use physlang_lsp::{
    completions_for_prefix, diagnostics_for_source, hover_for_position, CompletionKind,
    DiagnosticSeverity,
};
use physlang_runtime::compile_and_run;
use physlang_viz::{
    demo_gaussian_field, element_symbol, extract_slice, parse_structure,
    render_field_isosurface_png, render_field_slice_3d_png, render_field_slice_png,
    render_molecule_png, OrbitCamera, ScalarField, SliceAxis,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Serialize)]
pub struct IdeDiagnostic {
    line: u32,
    column: u32,
    message: String,
    severity: String,
}

#[derive(Serialize)]
pub struct IdeCompletion {
    label: String,
    detail: Option<String>,
    insert_text: String,
    kind: String,
}

#[derive(Serialize)]
pub struct IdeHover {
    contents: String,
}

#[derive(Serialize)]
pub struct RunResult {
    stdout: Vec<String>,
    result: Option<String>,
    error: Option<String>,
    backend: String,
}

#[derive(Serialize)]
pub struct ProjectFile {
    path: String,
    name: String,
    kind: String,
}

#[derive(Serialize)]
pub struct IdeAtom {
    element: u8,
    symbol: String,
    x: f64,
    y: f64,
    z: f64,
    radius: f64,
}

#[derive(Serialize)]
pub struct IdeMolecule {
    name: String,
    path: String,
    atoms: Vec<IdeAtom>,
    bonds: Vec<[usize; 2]>,
}

#[derive(Serialize, Deserialize)]
pub struct IdeCamera {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    pub width: u32,
    pub height: u32,
}

impl From<IdeCamera> for OrbitCamera {
    fn from(c: IdeCamera) -> Self {
        OrbitCamera {
            yaw: c.yaw,
            pitch: c.pitch,
            zoom: c.zoom,
            width: c.width.max(1),
            height: c.height.max(1),
            fov_y_deg: 45.0,
        }
    }
}

#[derive(Serialize)]
pub struct RenderFrameResult {
    png: Vec<u8>,
    backend: String,
}

#[derive(Serialize, Deserialize)]
pub struct IdeFieldSlice {
    name: String,
    path: String,
    axis: String,
    index: usize,
    width: usize,
    height: usize,
    values: Vec<f64>,
    min: f64,
    max: f64,
    depth: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    wgpu_png: Option<Vec<u8>>,
}

#[tauri::command]
fn read_text_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path))
}

#[tauri::command]
fn write_text_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, content).map_err(|e| format!("write {}: {e}", path))
}

#[tauri::command]
fn list_phys_files(root: String) -> Result<Vec<ProjectFile>, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(format!("not a directory: {root}"));
    }
    let mut files = Vec::new();
    collect_project_files(&root_path, &mut files).map_err(|e| e.to_string())?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

#[tauri::command]
fn parse_molecule_file(path: String) -> Result<IdeMolecule, String> {
    let source = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path))?;
    let mol = parse_structure(&source, Some(&path))?;
    Ok(molecule_to_ide(&path, mol))
}

#[tauri::command]
fn parse_molecule_xyz(source: String, name: Option<String>) -> Result<IdeMolecule, String> {
    let path = name.unwrap_or_else(|| "inline.xyz".into());
    let mol = parse_structure(&source, Some(&path))?;
    Ok(molecule_to_ide(&path, mol))
}

#[tauri::command]
fn load_field_file(path: String) -> Result<IdeFieldSlice, String> {
    let source = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path))?;
    let field: ScalarField =
        serde_json::from_str(&source).map_err(|e| format!("field json: {e}"))?;
    field_to_ide(&path, field, path.rsplit('/').next().unwrap_or("field").to_string())
}

#[tauri::command]
fn demo_scalar_field(resolution: Option<usize>) -> Result<IdeFieldSlice, String> {
    let n = resolution.unwrap_or(32).clamp(8, 64);
    let field = demo_gaussian_field(n);
    field_to_ide("demo://gaussian", field, format!("gaussian-{n}^3"))
}

#[tauri::command]
fn field_slice_at(path: String, index: usize) -> Result<IdeFieldSlice, String> {
    if path == "demo://gaussian" {
        let field = demo_gaussian_field(32);
        return field_to_ide_at_index(&path, field, "gaussian-32^3".into(), index);
    }
    let source = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path))?;
    let field: ScalarField = serde_json::from_str(&source).map_err(|e| format!("field json: {e}"))?;
    let name = Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("field")
        .to_string();
    field_to_ide_at_index(&path, field, name, index)
}

#[tauri::command]
fn render_molecule_frame(path: String, camera: IdeCamera) -> Result<RenderFrameResult, String> {
    let source = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path))?;
    let mol = parse_structure(&source, Some(&path))?;
    let cam: OrbitCamera = camera.into();
    let png = render_molecule_png(&mol, &cam)?;
    Ok(RenderFrameResult {
        png,
        backend: "wgpu-molecule".into(),
    })
}

#[tauri::command]
fn render_field_frame(
    path: String,
    index: usize,
    camera: IdeCamera,
    mode: Option<String>,
) -> Result<RenderFrameResult, String> {
    let field = load_scalar_field(&path)?;
    let cam: OrbitCamera = camera.into();
    let use_iso = mode.as_deref() == Some("isosurface");
    let png = if use_iso {
        render_field_isosurface_png(&field, 0.35, &cam)?
    } else {
        render_field_slice_3d_png(&field, index, &cam)?
    };
    Ok(RenderFrameResult {
        png,
        backend: if use_iso {
            "wgpu-isosurface".into()
        } else {
            "wgpu-field-slice-3d".into()
        },
    })
}

fn load_scalar_field(path: &str) -> Result<ScalarField, String> {
    if path == "demo://gaussian" {
        return Ok(demo_gaussian_field(32));
    }
    let source = fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    serde_json::from_str(&source).map_err(|e| format!("field json: {e}"))
}

fn field_to_ide(path: &str, field: ScalarField, name: String) -> Result<IdeFieldSlice, String> {
    let z_mid = field.shape[2] / 2;
    field_to_ide_at_index(path, field, name, z_mid)
}

fn field_to_ide_at_index(
    path: &str,
    field: ScalarField,
    name: String,
    z_index: usize,
) -> Result<IdeFieldSlice, String> {
    let slice = extract_slice(&field, SliceAxis::Z, z_index)?;
    let wgpu_png = render_field_slice_png(&field, z_index).ok();
    Ok(IdeFieldSlice {
        name,
        path: path.to_string(),
        axis: "Z".into(),
        index: z_index,
        width: slice.width,
        height: slice.height,
        values: slice.values,
        min: slice.min,
        max: slice.max,
        depth: field.shape[2],
        wgpu_png,
    })
}

fn molecule_to_ide(path: &str, mol: physlang_viz::MoleculeGeometry) -> IdeMolecule {
    IdeMolecule {
        name: mol.name,
        path: path.to_string(),
        atoms: mol
            .atoms
            .iter()
            .map(|a| IdeAtom {
                element: a.element,
                symbol: element_symbol(a.element).to_string(),
                x: a.x,
                y: a.y,
                z: a.z,
                radius: a.radius,
            })
            .collect(),
        bonds: mol.bonds,
    }
}

fn collect_project_files(dir: &Path, out: &mut Vec<ProjectFile>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if name.starts_with('.') || name == "node_modules" || name == "target" {
                continue;
            }
            collect_project_files(&path, out)?;
        } else {
            let fname = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            let kind = if fname.ends_with(".field.json") {
                "field"
            } else {
                match path.extension().and_then(|e| e.to_str()) {
                    Some("phys") => "phys",
                    Some("xyz") => "xyz",
                    Some("pdb") => "pdb",
                    _ => continue,
                }
            };
            if let Some(path_str) = path.to_str() {
                out.push(ProjectFile {
                    name: path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("file")
                        .to_string(),
                    path: path_str.to_string(),
                    kind: kind.into(),
                });
            }
        }
    }
    Ok(())
}

fn severity_name(s: DiagnosticSeverity) -> &'static str {
    match s {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Info => "info",
    }
}

fn completion_kind_name(k: CompletionKind) -> &'static str {
    match k {
        CompletionKind::Keyword => "keyword",
        CompletionKind::Type => "type",
        CompletionKind::Gate => "function",
        CompletionKind::Function => "function",
        CompletionKind::Attribute => "property",
    }
}

#[tauri::command]
fn check_phys_source(source: String) -> Vec<IdeDiagnostic> {
    diagnostics_for_source(&source)
        .into_iter()
        .map(|d| IdeDiagnostic {
            line: d.line,
            column: d.column,
            message: d.message,
            severity: severity_name(d.severity).to_string(),
        })
        .collect()
}

#[tauri::command]
fn complete_phys_prefix(prefix: String) -> Vec<IdeCompletion> {
    completions_for_prefix(&prefix)
        .into_iter()
        .map(|c| IdeCompletion {
            label: c.label,
            detail: c.detail,
            insert_text: c.insert_text,
            kind: completion_kind_name(c.kind).to_string(),
        })
        .collect()
}

#[tauri::command]
fn hover_phys_source(source: String, line: u32, column: u32) -> Option<IdeHover> {
    hover_for_position(&source, line, column).map(|h| IdeHover {
        contents: h.contents,
    })
}

#[tauri::command]
fn run_phys_source(source: String, entry: Option<String>) -> RunResult {
    let entry = entry.as_deref().unwrap_or("main");
    match compile_and_run(&source, entry) {
        Ok(out) => RunResult {
            stdout: out.stdout,
            result: out.return_value.map(|v| v.display()),
            error: None,
            backend: "physlang-runtime".into(),
        },
        Err(e) => RunResult {
            stdout: vec![],
            result: None,
            error: Some(e.to_string()),
            backend: "physlang-runtime".into(),
        },
    }
}

#[tauri::command]
fn run_phys_file(
    path: String,
    entry: Option<String>,
    project_root: Option<String>,
) -> RunResult {
    let source = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            return RunResult {
                stdout: vec![],
                result: None,
                error: Some(format!("read {}: {e}", path)),
                backend: "physlang-runtime".into(),
            };
        }
    };

    let entry = entry.as_deref().unwrap_or("main");
    match compile_and_run(&source, entry) {
        Ok(out) => RunResult {
            stdout: out.stdout,
            result: out.return_value.map(|v| v.display()),
            error: None,
            backend: "physlang-runtime".into(),
        },
        Err(native_err) => {
            if let Some(fallback) = run_phys_cli_fallback(&path, project_root.as_deref()) {
                fallback
            } else {
                RunResult {
                    stdout: vec![],
                    result: None,
                    error: Some(native_err.to_string()),
                    backend: "physlang-runtime".into(),
                }
            }
        }
    }
}

fn run_phys_cli_fallback(path: &str, project_root: Option<&str>) -> Option<RunResult> {
    if let Some(result) = try_phys_binary(path) {
        return Some(result);
    }
    if let Some(root) = project_root {
        if let Some(result) = try_cargo_run(path, root) {
            return Some(result);
        }
    }
    try_python_run(path)
}

fn try_phys_binary(path: &str) -> Option<RunResult> {
    let output = Command::new("phys")
        .args(["run", path])
        .output()
        .ok()?;
    Some(cli_output_to_result(output, "phys run"))
}

fn try_cargo_run(path: &str, project_root: &str) -> Option<RunResult> {
    let manifest = Path::new(project_root).join("physlang/physlang-cli/Cargo.toml");
    if !manifest.exists() {
        let alt = Path::new(project_root).join("Cargo.toml");
        if !alt.exists() {
            return None;
        }
    }
    let mut cmd = Command::new("cargo");
    cmd.current_dir(project_root);
    if manifest.exists() {
        cmd.args([
            "run",
            "--manifest-path",
            manifest.to_str()?,
            "--",
            "run",
            path,
        ]);
    } else {
        cmd.args(["run", "--", "run", path]);
    }
    let output = cmd.output().ok()?;
    Some(cli_output_to_result(output, "cargo run -- run"))
}

fn try_python_run(path: &str) -> Option<RunResult> {
    let script = format!(
        "import json, physlang; print(json.dumps(physlang.run({path:?})))",
        path = path
    );
    let output = Command::new("python")
        .args(["-c", &script])
        .output()
        .ok()?;
    if !output.status.success() {
        return Some(cli_output_to_result(output, "python physlang.run()"));
    }
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(stdout.trim()) {
        let lines = value
            .get("stdout")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let result = value
            .get("result")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        return Some(RunResult {
            stdout: lines,
            result,
            error: None,
            backend: "python physlang.run()".into(),
        });
    }
    Some(cli_output_to_result(output, "python physlang.run()"))
}

fn cli_output_to_result(
    output: std::process::Output,
    backend: &str,
) -> RunResult {
    let stdout = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() {
        let mut result = None;
        if let Some(line) = stdout.iter().find(|l| l.starts_with("=> ")) {
            result = Some(line.trim_start_matches("=> ").to_string());
        }
        RunResult {
            stdout,
            result,
            error: if stderr.is_empty() {
                None
            } else {
                Some(stderr)
            },
            backend: backend.into(),
        }
    } else {
        RunResult {
            stdout,
            result: None,
            error: Some(if stderr.is_empty() {
                format!("exit code {:?}", output.status.code())
            } else {
                stderr
            }),
            backend: backend.into(),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            read_text_file,
            write_text_file,
            list_phys_files,
            parse_molecule_file,
            parse_molecule_xyz,
            load_field_file,
            demo_scalar_field,
            field_slice_at,
            render_molecule_frame,
            render_field_frame,
            check_phys_source,
            complete_phys_prefix,
            hover_phys_source,
            run_phys_source,
            run_phys_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PhysicsLang IDE");
}
