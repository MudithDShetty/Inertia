//! Submit external quantum-chemistry jobs (Gaussian, ORCA) from the IDE.

use serde::Serialize;
use std::collections::VecDeque;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;

#[derive(Debug, Clone, Serialize)]
pub struct ChemBackendInfo {
    pub id: String,
    pub executable: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChemJobResult {
    pub success: bool,
    pub command: String,
    pub log_path: Option<String>,
    pub stdout: String,
    pub stderr: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChemJobQueueItem {
    pub backend: String,
    pub input_path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChemJobProgress {
    pub running: bool,
    pub backend: String,
    pub input_path: String,
    pub message: String,
    pub stderr_tail: String,
    pub queue: Vec<ChemJobQueueItem>,
    pub last_completed_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChemJobEnqueueResult {
    pub queued: bool,
    pub queue_position: usize,
    pub message: String,
}

struct JobManager {
    progress: ChemJobProgress,
    queue: VecDeque<(String, String)>,
    child: Option<Child>,
    cancel_flag: AtomicBool,
    worker_started: bool,
    last_result: Option<ChemJobResult>,
    last_completed_path: Option<String>,
}

static JOB_MANAGER: OnceLock<Mutex<JobManager>> = OnceLock::new();

fn manager() -> &'static Mutex<JobManager> {
    JOB_MANAGER.get_or_init(|| {
        Mutex::new(JobManager {
            progress: ChemJobProgress {
                running: false,
                backend: String::new(),
                input_path: String::new(),
                message: String::new(),
                stderr_tail: String::new(),
                queue: vec![],
                last_completed_path: None,
            },
            queue: VecDeque::new(),
            child: None,
            cancel_flag: AtomicBool::new(false),
            worker_started: false,
            last_result: None,
            last_completed_path: None,
        })
    })
}

fn sync_queue_view(m: &mut JobManager) {
    m.progress.queue = m
        .queue
        .iter()
        .map(|(backend, input_path)| ChemJobQueueItem {
            backend: backend.clone(),
            input_path: input_path.clone(),
        })
        .collect();
}

fn append_stderr_tail(m: &mut JobManager, chunk: &str) {
    if chunk.is_empty() {
        return;
    }
    m.progress.stderr_tail.push_str(chunk);
    const MAX: usize = 16_384;
    if m.progress.stderr_tail.len() > MAX {
        let drain = m.progress.stderr_tail.len() - MAX;
        m.progress.stderr_tail.drain(..drain);
    }
}

fn tail_lines(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    if lines.len() <= n {
        return s.trim().to_string();
    }
    lines[lines.len() - n..].join("\n")
}

pub fn chem_job_progress() -> ChemJobProgress {
    let m = manager().lock().unwrap();
    m.progress.clone()
}

pub fn chem_job_last_result() -> Option<ChemJobResult> {
    manager().lock().unwrap().last_result.clone()
}

pub fn chem_job_cancel() -> Result<String, String> {
    let mut m = manager().lock().unwrap();
    if !m.progress.running {
        return Err("no job running".into());
    }
    m.cancel_flag.store(true, Ordering::SeqCst);
    if let Some(mut child) = m.child.take() {
        let _ = child.kill();
    }
    m.progress.message = "Cancelling…".into();
    Ok("cancel requested".into())
}

fn ensure_worker() {
    let mut m = manager().lock().unwrap();
    if m.worker_started {
        return;
    }
    m.worker_started = true;
    drop(m);
    thread::spawn(worker_loop);
}

fn worker_loop() {
    loop {
        let next = {
            let mut m = manager().lock().unwrap();
            if m.progress.running {
                drop(m);
                thread::sleep(std::time::Duration::from_millis(100));
                continue;
            }
            let item = m.queue.pop_front();
            sync_queue_view(&mut m);
            item
        };
        let Some((backend, path)) = next else {
            thread::sleep(std::time::Duration::from_millis(200));
            continue;
        };

        {
            let mut m = manager().lock().unwrap();
            m.cancel_flag.store(false, Ordering::SeqCst);
            m.progress.running = true;
            m.progress.backend = backend.clone();
            m.progress.input_path = path.clone();
            m.progress.message = format!("Running {}…", backend_label(&backend));
            m.progress.stderr_tail.clear();
            sync_queue_view(&mut m);
        }

        let result = if backend == "gaussian" {
            run_gaussian_job(&path)
        } else if backend == "orca" {
            run_orca_job(&path)
        } else {
            Err(format!("unknown backend: {backend}"))
        };

        let mut job_result = match result {
            Ok(r) => r,
            Err(e) => ChemJobResult {
                success: false,
                command: String::new(),
                log_path: None,
                stdout: String::new(),
                stderr: String::new(),
                message: e,
            },
        };

        let cancelled = manager()
            .lock()
            .unwrap()
            .cancel_flag
            .load(Ordering::SeqCst);

        let mut m = manager().lock().unwrap();
        m.child = None;
        m.progress.running = false;
        if cancelled {
            m.progress.message = "Cancelled".into();
            job_result.message = "Job cancelled".into();
        } else {
            m.progress.message = job_result.message.clone();
        }
        m.progress.stderr_tail = tail_lines(&job_result.stderr, 24);
        sync_queue_view(&mut m);
        m.last_result = Some(job_result.clone());
        m.last_completed_path = Some(path.clone());
        m.progress.last_completed_path = Some(path);
    }
}

fn backend_label(id: &str) -> &str {
    match id {
        "gaussian" => "Gaussian",
        "orca" => "ORCA",
        _ => id,
    }
}

pub fn list_chem_backends() -> Vec<ChemBackendInfo> {
    let mut out = Vec::new();
    if let Some(exe) = find_gaussian() {
        out.push(ChemBackendInfo {
            id: "gaussian".into(),
            executable: exe.display().to_string(),
        });
    }
    if let Some(exe) = find_orca() {
        out.push(ChemBackendInfo {
            id: "orca".into(),
            executable: exe.display().to_string(),
        });
    }
    out
}

pub fn enqueue_chem_job(backend: &str, gjf_path: &str) -> Result<ChemJobEnqueueResult, String> {
    let input = Path::new(gjf_path);
    if !input.is_file() {
        return Err(format!("input not found: {gjf_path}"));
    }
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext != "gjf" && ext != "com" {
        return Err("job requires a .gjf or .com file".to_string());
    }
    match backend {
        "gaussian" if find_gaussian().is_none() => {
            return Err(
                "Gaussian not found. Set GAUSSIAN_EXE or install g16/g09".into(),
            );
        }
        "orca" if find_orca().is_none() => {
            return Err("ORCA not found. Set ORCA_EXE or add orca to PATH".into());
        }
        "gaussian" | "orca" => {}
        other => return Err(format!("unknown backend: {other}")),
    }

    let mut m = manager().lock().unwrap();
    let was_running = m.progress.running;
    let position = m.queue.len();
    m.queue.push_back((backend.to_string(), gjf_path.to_string()));
    sync_queue_view(&mut m);
    drop(m);

    ensure_worker();

    Ok(ChemJobEnqueueResult {
        queued: was_running || position > 0,
        queue_position: position,
        message: if was_running || position > 0 {
            format!("Queued ({})", position + 1)
        } else {
            format!("Started {} job", backend_label(backend))
        },
    })
}

pub fn submit_gaussian_job(gjf_path: &str) -> Result<ChemJobResult, String> {
    enqueue_chem_job("gaussian", gjf_path)?;
    wait_for_job(gjf_path)
}

pub fn submit_orca_job(gjf_path: &str) -> Result<ChemJobResult, String> {
    enqueue_chem_job("orca", gjf_path)?;
    wait_for_job(gjf_path)
}

fn wait_for_job(input_path: &str) -> Result<ChemJobResult, String> {
    loop {
        thread::sleep(std::time::Duration::from_millis(200));
        let m = manager().lock().unwrap();
        if !m.progress.running
            && m.last_completed_path.as_deref() == Some(input_path)
        {
            return m
                .last_result
                .clone()
                .ok_or_else(|| "job finished without result".into());
        }
        if !m.progress.running && m.queue.is_empty() && m.last_completed_path.is_none() {
            return Err("job did not start".into());
        }
    }
}

fn run_gaussian_job(gjf_path: &str) -> Result<ChemJobResult, String> {
    let input = Path::new(gjf_path);
    let exe = find_gaussian().ok_or_else(|| "Gaussian executable missing".to_string())?;
    let work_dir = input.parent().unwrap_or_else(|| Path::new("."));
    let log_path = input.with_extension("log");
    let log_path_str = log_path.to_str().ok_or("invalid log path")?;

    let mut cmd = Command::new(&exe);
    cmd.current_dir(work_dir)
        .stdin(fs::File::open(input).map_err(|e| format!("open input: {e}"))?)
        .stdout(
            fs::File::create(&log_path)
                .map_err(|e| format!("create log {}: {e}", log_path.display()))?,
        )
        .stderr(Stdio::piped());

    let command_line = format!(
        "cd {} && {} < {}",
        work_dir.display(),
        exe.display(),
        input.file_name().unwrap_or_default().to_string_lossy()
    );

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to run Gaussian: {e}"))?;

    let stderr = child.stderr.take();
    {
        let mut m = manager().lock().unwrap();
        m.child = Some(child);
    }

    let stderr_text = read_stderr_live(stderr);

    let status = {
        let mut m = manager().lock().unwrap();
        let mut child = m.child.take().expect("child");
        let status = child.wait().map_err(|e| format!("wait Gaussian: {e}"))?;
        status
    };

    let cancelled = manager()
        .lock()
        .unwrap()
        .cancel_flag
        .load(Ordering::SeqCst);

    let success = !cancelled && (status.success() || log_path.is_file());
    let message = if cancelled {
        "Job cancelled".into()
    } else if success {
        format!("Gaussian finished — log: {log_path_str}")
    } else {
        format!(
            "Gaussian exited with {} — check {}",
            status.code().unwrap_or(-1),
            log_path_str
        )
    };

    Ok(ChemJobResult {
        success,
        command: command_line,
        log_path: Some(log_path_str.to_string()),
        stdout: String::new(),
        stderr: stderr_text,
        message,
    })
}

fn run_orca_job(gjf_path: &str) -> Result<ChemJobResult, String> {
    let input = Path::new(gjf_path);
    let exe = find_orca().ok_or_else(|| "ORCA executable missing".to_string())?;
    let work_dir = input.parent().unwrap_or_else(|| Path::new("."));
    let inp_path = input.with_extension("inp");
    let log_path = input.with_extension("out");

    let gjf_text = fs::read_to_string(input).map_err(|e| format!("read gjf: {e}"))?;
    let orca_inp = gjf_to_orca(&gjf_text);
    fs::write(&inp_path, &orca_inp).map_err(|e| format!("write inp: {e}"))?;

    let inp_name = inp_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    let mut cmd = Command::new(&exe);
    cmd.current_dir(work_dir)
        .arg(&inp_name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let command_line = format!("{} {inp_name}", exe.display());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to run ORCA: {e}"))?;

    let stderr = child.stderr.take();
    let stdout = child.stdout.take();
    {
        let mut m = manager().lock().unwrap();
        m.child = Some(child);
    }

    let stderr_text = read_stderr_live(stderr);
    let stdout_text = read_stdout_live(stdout);

    let status = {
        let mut m = manager().lock().unwrap();
        let mut child = m.child.take().expect("child");
        child.wait().map_err(|e| format!("wait ORCA: {e}"))?
    };

    let cancelled = manager()
        .lock()
        .unwrap()
        .cancel_flag
        .load(Ordering::SeqCst);

    let log_path_str = log_path.to_str().map(|s| s.to_string());
    let success = !cancelled && (status.success() || log_path.is_file());
    let message = if cancelled {
        "Job cancelled".into()
    } else if success {
        format!(
            "ORCA finished — {}",
            log_path_str.as_deref().unwrap_or("see stdout")
        )
    } else {
        format!("ORCA exited with {}", status.code().unwrap_or(-1))
    };

    Ok(ChemJobResult {
        success,
        command: command_line,
        log_path: log_path_str,
        stdout: stdout_text,
        stderr: stderr_text,
        message,
    })
}

fn read_stderr_live(stderr: Option<std::process::ChildStderr>) -> String {
    let Some(stderr) = stderr else {
        return String::new();
    };
    let reader = BufReader::new(stderr);
    let mut out = String::new();
    for line in reader.lines().map_while(Result::ok) {
        let chunk = format!("{line}\n");
        out.push_str(&chunk);
        if let Ok(mut m) = manager().lock() {
            append_stderr_tail(&mut m, &chunk);
        }
    }
    out
}

fn read_stdout_live(stdout: Option<std::process::ChildStdout>) -> String {
    let Some(stdout) = stdout else {
        return String::new();
    };
    let reader = BufReader::new(stdout);
    reader
        .lines()
        .map_while(Result::ok)
        .collect::<Vec<_>>()
        .join("\n")
}

fn gjf_to_orca(gjf: &str) -> String {
    let mut lines: Vec<&str> = gjf.lines().collect();
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    if lines.is_empty() {
        return "! HF def2-SVP\n\n* xyz 0 1\n*\n".into();
    }

    let route = lines[0].trim().trim_start_matches('#').trim();
    let charge_mult = lines.get(2).copied().unwrap_or("0 1");
    let cm: Vec<&str> = charge_mult.split_whitespace().collect();
    let charge = cm.first().copied().unwrap_or("0");
    let mult = cm.get(1).copied().unwrap_or("1");

    let mut coords: Vec<&str> = Vec::new();
    for line in lines.iter().skip(3) {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
            coords.push(t);
        }
    }

    let mut out = String::new();
    out.push_str("! ");
    out.push_str(if route.is_empty() { "HF def2-SVP" } else { route });
    out.push_str("\n\n* xyz ");
    out.push_str(charge);
    out.push(' ');
    out.push_str(mult);
    out.push('\n');
    for line in coords {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str("*\n");
    out
}

fn find_gaussian() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("GAUSSIAN_EXE") {
        let path = PathBuf::from(&p);
        if path.is_file() {
            return Some(path);
        }
    }
    if let Ok(root) = std::env::var("g16root") {
        for name in ["g16.exe", "g16"] {
            let candidate = PathBuf::from(&root).join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    for name in ["g16", "g09", "g16.exe", "g09.exe"] {
        if let Some(p) = find_on_path(name) {
            return Some(p);
        }
    }
    None
}

fn find_orca() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("ORCA_EXE") {
        let path = PathBuf::from(&p);
        if path.is_file() {
            return Some(path);
        }
    }
    for name in ["orca", "orca.exe"] {
        if let Some(p) = find_on_path(name) {
            return Some(p);
        }
    }
    None
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    #[cfg(windows)]
    let which_cmd = "where";
    #[cfg(not(windows))]
    let which_cmd = "which";

    let output = Command::new(which_cmd).arg(name).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let line = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()?
        .trim()
        .to_string();
    if line.is_empty() {
        return None;
    }
    let path = PathBuf::from(line);
    if path.is_file() {
        Some(path)
    } else {
        None
    }
}
