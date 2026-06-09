use physlang_lsp::{
    code_actions_at, completions_for_prefix, definition_at_with_stdlib, diagnostics_for_source,
    doc_lines_before, find_references_at, find_stdlib_dir, generate_stdlib_markdown,
    hover_for_position, index_stdlib_dir, rename_at, CompletionKind, DiagnosticSeverity,
};
use physlang_runtime::compile_and_run;
use physlang_viz::{
    animate_geometry, cube_to_scalar_field, demo_gaussian_field, element_symbol, extract_slice,
    fchk_density_field, fchk_esp_field, fchk_mo_field, parse_cube, parse_fchk, parse_gaussian_log,
    parse_gjf, parse_log_vibrations, parse_structure, pick_molecule_atom,
    extract_gjf_coordinate_block, replace_gjf_coordinate_block,
    render_field_isosurface_png, render_field_mo_isosurface_png, render_field_slice_3d_png,
    render_field_slice_png, render_field_volume_png, render_molecule_png, scalar_field_to_cube,
    scalar_field_to_vtk, ConvergencePlot, MolRenderStyle, OrbitCamera, ScalarField, SliceAxis,
    VibrationData,
};
mod chem_jobs;

use chem_jobs::{
    chem_job_cancel, chem_job_last_result as last_chem_job_result, chem_job_progress, enqueue_chem_job,
    list_chem_backends, ChemBackendInfo, ChemJobEnqueueResult, ChemJobProgress, ChemJobResult,
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
pub struct IdeLocation {
    line: u32,
    column: u32,
    end_column: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<String>,
}

#[derive(Serialize)]
pub struct IdeCodeAction {
    title: String,
    edits: Vec<IdeTextEdit>,
}

#[derive(Serialize)]
pub struct IdeTextEdit {
    line: u32,
    column: u32,
    end_column: u32,
    new_text: String,
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
pub struct IdeChemMeta {
    format: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    charge: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    multiplicity: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    coordinate_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    final_energy_hartree: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scf_cycles: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    n_frequencies: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    has_density: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    has_mos: Option<bool>,
}

#[derive(Serialize)]
pub struct IdeMolecule {
    name: String,
    path: String,
    atoms: Vec<IdeAtom>,
    bonds: Vec<[usize; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chem: Option<IdeChemMeta>,
}

#[derive(Serialize)]
pub struct IdeVibrationMode {
    index: usize,
    frequency_cm1: f64,
}

#[derive(Serialize)]
pub struct IdeVibrationInfo {
    path: String,
    modes: Vec<IdeVibrationMode>,
}

#[derive(Serialize, Deserialize, Clone)]
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
fn write_binary_file(path: String, data: Vec<u8>) -> Result<(), String> {
    fs::write(&path, &data).map_err(|e| format!("write {}: {e}", path))
}

#[tauri::command]
fn list_phys_files(root: String) -> Result<Vec<ProjectFile>, String> {
    let root_path = PathBuf::from(&root);
    if !root_path.is_dir() {
        return Err(format!("not a directory: {root}"));
    }
    let mut files = Vec::new();
    collect_project_files(&root_path, &mut files).map_err(|e| e.to_string())?;
    append_stdlib_files(&root_path, &mut files);
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

#[derive(Serialize, Deserialize, Clone)]
struct PackageCatalogEntry {
    id: String,
    name: String,
    description: String,
    source: String,
    #[serde(default)]
    builtin: bool,
}

#[derive(Deserialize)]
struct PackageCatalog {
    packages: Vec<PackageCatalogEntry>,
}

fn find_catalog_path(start: &Path) -> Option<PathBuf> {
    let mut cur = Some(start);
    while let Some(dir) = cur {
        let candidate = dir.join("packages").join("catalog.json");
        if candidate.is_file() {
            return Some(candidate);
        }
        cur = dir.parent();
    }
    None
}

#[tauri::command]
fn list_package_catalog(root: String) -> Result<Vec<PackageCatalogEntry>, String> {
    let catalog_path =
        find_catalog_path(Path::new(&root)).ok_or("packages/catalog.json not found")?;
    let text = fs::read_to_string(&catalog_path).map_err(|e| e.to_string())?;
    let catalog: PackageCatalog = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(catalog.packages)
}

#[tauri::command]
fn list_installed_packages(root: String) -> Result<Vec<String>, String> {
    let dir = Path::new(&root).join(".inertia").join("packages");
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let mut ids = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("phys") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                ids.push(stem.to_string());
            }
        }
    }
    ids.sort();
    Ok(ids)
}

#[tauri::command]
fn install_package(root: String, package_id: String) -> Result<String, String> {
    let root_path = Path::new(&root);
    let catalog_path =
        find_catalog_path(root_path).ok_or("packages/catalog.json not found")?;
    let repo_root = catalog_path
        .parent()
        .and_then(|p| p.parent())
        .ok_or("invalid catalog path")?;
    let text = fs::read_to_string(&catalog_path).map_err(|e| e.to_string())?;
    let catalog: PackageCatalog = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let pkg = catalog
        .packages
        .iter()
        .find(|p| p.id == package_id)
        .ok_or_else(|| format!("unknown package: {package_id}"))?;
    let src = repo_root.join(&pkg.source);
    if !src.is_file() {
        return Err(format!("package source missing: {}", src.display()));
    }
    let dest_dir = root_path.join(".inertia").join("packages");
    fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    let dest = dest_dir.join(format!("{package_id}.phys"));
    fs::copy(&src, &dest).map_err(|e| e.to_string())?;
    Ok(format!("Installed {} → {}", pkg.name, dest.display()))
}

fn append_stdlib_files(root: &Path, out: &mut Vec<ProjectFile>) {
    let Some(stdlib_dir) = find_stdlib_dir(root) else {
        return;
    };
    let Ok(entries) = fs::read_dir(&stdlib_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("phys") {
            continue;
        }
        let Some(path_str) = path.to_str() else {
            continue;
        };
        if out.iter().any(|f| f.path == path_str) {
            continue;
        }
        let fname = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file.phys");
        out.push(ProjectFile {
            name: format!("stdlib/{fname}"),
            path: path_str.to_string(),
            kind: "phys".into(),
        });
    }
}

#[tauri::command]
fn parse_molecule_file(path: String) -> Result<IdeMolecule, String> {
    let source = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path))?;
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".gjf") || lower.ends_with(".com") {
        let job = parse_gjf(&source)?;
        return Ok(molecule_to_ide_chem(&path, job));
    }
    if lower.ends_with(".log") {
        let log = parse_gaussian_log(&source)?;
        return Ok(molecule_to_ide_log(&path, log));
    }
    if lower.ends_with(".fchk") {
        let fchk = parse_fchk(&source)?;
        return Ok(molecule_to_ide_fchk(&path, fchk));
    }
    let mol = parse_structure(&source, Some(&path))?;
    Ok(molecule_to_ide(&path, mol))
}

#[tauri::command]
fn parse_gaussian_log_file(path: String) -> Result<IdeMolecule, String> {
    let source = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path))?;
    let log = parse_gaussian_log(&source)?;
    Ok(molecule_to_ide_log(&path, log))
}

#[tauri::command]
fn load_cube_file(path: String) -> Result<IdeFieldSlice, String> {
    let source = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path))?;
    let cube = parse_cube(&source)?;
    let name = Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("cube")
        .to_string();
    let title = if cube.title[0].is_empty() {
        name.clone()
    } else {
        cube.title[0].clone()
    };
    field_to_ide(&path, cube_to_scalar_field(&cube), title)
}

#[tauri::command]
fn list_vibration_modes(path: String) -> Result<IdeVibrationInfo, String> {
    let vib = load_vibration_data(&path)?;
    Ok(IdeVibrationInfo {
        path,
        modes: vib
            .modes
            .iter()
            .map(|m| IdeVibrationMode {
                index: m.index,
                frequency_cm1: m.frequency_cm1,
            })
            .collect(),
    })
}

#[tauri::command]
fn vibration_frame(path: String, mode_index: usize, phase: f64) -> Result<IdeMolecule, String> {
    let vib = load_vibration_data(&path)?;
    let mode = vib
        .modes
        .get(mode_index)
        .ok_or_else(|| format!("mode index {mode_index} out of range"))?;
    let animated = animate_geometry(&vib.equilibrium, mode, phase, 1.0);
    Ok(molecule_to_ide(&path, animated))
}

fn load_vibration_data(path: &str) -> Result<VibrationData, String> {
    let source = fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".fchk") {
        let fchk = parse_fchk(&source)?;
        return Ok(vibration_data_from_fchk(fchk));
    }
    if lower.ends_with(".log") {
        let log = parse_gaussian_log(&source)?;
        let geo = log
            .geometry
            .ok_or_else(|| "log: no geometry for vibration".to_string())?;
        return Ok(parse_log_vibrations(&source, &geo));
    }
    Err("vibration: need .log or .fchk file".to_string())
}

fn vibration_data_from_fchk(fchk: physlang_viz::FchkFile) -> VibrationData {
    use physlang_viz::NormalMode;
    let geo = fchk.geometry.clone();
    let modes: Vec<NormalMode> = if fchk.vibrational_frequencies_cm1.is_empty() {
        parse_log_vibrations("", &geo).modes
    } else {
        fchk.vibrational_frequencies_cm1
            .iter()
            .enumerate()
            .map(|(i, &f)| {
                let stub = parse_log_vibrations("", &geo);
                let disp = stub
                    .modes
                    .get(i)
                    .map(|m| m.displacements.clone())
                    .unwrap_or_default();
                NormalMode {
                    index: i,
                    frequency_cm1: f,
                    displacements: disp,
                }
            })
            .collect()
    };
    VibrationData {
        equilibrium: geo,
        modes,
    }
}

#[tauri::command]
fn parse_fchk_file(path: String) -> Result<IdeMolecule, String> {
    let source = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path))?;
    let fchk = parse_fchk(&source)?;
    Ok(molecule_to_ide_fchk(&path, fchk))
}

#[tauri::command]
fn parse_molecule_xyz(source: String, name: Option<String>) -> Result<IdeMolecule, String> {
    let path = name.unwrap_or_else(|| "inline.xyz".into());
    let mol = parse_structure(&source, Some(&path))?;
    Ok(molecule_to_ide(&path, mol))
}

fn companion_cube_path(fchk_path: &str) -> Option<PathBuf> {
    let path = Path::new(fchk_path);
    let parent = path.parent()?;
    let stem = path.file_stem()?.to_str()?;
    for name in [
        format!("{stem}.cube"),
        format!("{stem}_density.cube"),
        format!("{stem}-density.cube"),
    ] {
        let candidate = parent.join(&name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[tauri::command]
fn load_fchk_density_file(path: String) -> Result<IdeFieldSlice, String> {
    if let Some(cube_path) = companion_cube_path(&path) {
        let cube_str = cube_path.to_str().ok_or("invalid cube path")?;
        return load_cube_file(cube_str.to_string());
    }
    let source = fs::read_to_string(&path).map_err(|e| format!("read {path}: {e}"))?;
    let fchk = parse_fchk(&source)?;
    let field = fchk_density_field(&fchk, 32);
    let density_kind = if fchk.scf_density.is_some()
        && fchk.shell_types.is_some()
        && fchk.primitive_exponents.is_some()
    {
        "GTO"
    } else {
        "promolecule"
    };
    let name = format!("{} density ({density_kind})", fchk.title);
    field_to_ide(&format!("{path}|density"), field, name)
}

#[derive(Serialize)]
struct FchkMoInfo {
    index: usize,
    label: String,
    energy_hartree: Option<f64>,
    occupied: bool,
}

#[tauri::command]
fn fchk_list_mos(path: String) -> Result<Vec<FchkMoInfo>, String> {
    let (_, fchk) = load_fchk_source(&path)?;
    let n_mos = fchk.n_mos().ok_or("fchk: no Alpha MO coefficients")?;
    let homo = fchk.homo_index();
    let energies = fchk.orbital_energies.as_deref();
    let mut out = Vec::with_capacity(n_mos);
    for i in 0..n_mos {
        let gaussian_n = i + 1;
        let label = match (homo, fchk.lumo_index()) {
            (Some(h), Some(l)) if i == h => format!("HOMO ({gaussian_n})"),
            (_, Some(l)) if i == l => format!("LUMO ({gaussian_n})"),
            _ => format!("MO {gaussian_n}"),
        };
        let occupied = homo.map(|h| i <= h).unwrap_or(false);
        let energy_hartree = energies.and_then(|e| e.get(i).copied());
        out.push(FchkMoInfo {
            index: i,
            label,
            energy_hartree,
            occupied,
        });
    }
    Ok(out)
}

#[tauri::command]
fn load_fchk_mo_file(path: String, mo_index: usize) -> Result<IdeFieldSlice, String> {
    let (_, fchk) = load_fchk_source(&path)?;
    let field = fchk_mo_field(&fchk, mo_index, 32)?;
    let gaussian_n = mo_index + 1;
    let tag = match (fchk.homo_index(), fchk.lumo_index()) {
        (Some(h), Some(l)) if mo_index == h => "HOMO".into(),
        (_, Some(l)) if mo_index == l => "LUMO".into(),
        _ => format!("MO {gaussian_n}"),
    };
    let energy = fchk
        .orbital_energies
        .as_ref()
        .and_then(|e| e.get(mo_index))
        .map(|e| format!(" E={e:.4} Ha"))
        .unwrap_or_default();
    let name = format!("{} {tag}{energy}", fchk.title);
    field_to_ide(&format!("{path}|mo:{mo_index}"), field, name)
}

#[tauri::command]
fn load_fchk_esp_file(path: String) -> Result<IdeFieldSlice, String> {
    let (_, fchk) = load_fchk_source(&path)?;
    let kind = if can_quantum_esp_fchk(&fchk) {
        "quantum Hartree"
    } else if fchk.mulliken_charges.is_some() {
        "Mulliken monopole"
    } else {
        "nuclear"
    };
    let field = fchk_esp_field(&fchk, 32);
    let name = format!("{} ESP ({kind})", fchk.title);
    field_to_ide(&format!("{path}|esp"), field, name)
}

fn can_quantum_esp_fchk(fchk: &physlang_viz::FchkFile) -> bool {
    use physlang_viz::basis_from_fchk;
    if basis_from_fchk(fchk).is_none() {
        return false;
    }
    let Some(scf) = fchk.scf_density.as_ref() else {
        return false;
    };
    let Some(n_basis) = fchk.n_basis else {
        return false;
    };
    n_basis > 0 && scf.len() >= n_basis * (n_basis + 1) / 2
}

#[tauri::command]
fn export_fchk_density_cube(path: String) -> Result<String, String> {
    let source = fs::read_to_string(&path).map_err(|e| format!("read {path}: {e}"))?;
    let fchk = parse_fchk(&source)?;
    let field = fchk_density_field(&fchk, 32);
    let cube_text = scalar_field_to_cube(&field, &format!("{} density", fchk.title));
    let out_path = Path::new(&path).with_extension("density.cube");
    fs::write(&out_path, &cube_text).map_err(|e| format!("write {}: {e}", out_path.display()))?;
    out_path
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "invalid output path".into())
}

#[tauri::command]
fn export_field_vtk(path: String) -> Result<String, String> {
    let field = load_scalar_field(&path)?;
    let base = path.split('|').next().unwrap_or(&path);
    let stem = Path::new(base)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("field");
    let vtk = scalar_field_to_vtk(&field, &format!("{stem} scalar field"));
    let out_path = if path.contains('|') {
        Path::new(base).with_extension("field.vtk")
    } else {
        Path::new(base).with_extension("vtk")
    };
    fs::write(&out_path, &vtk).map_err(|e| format!("write {}: {e}", out_path.display()))?;
    out_path
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "invalid output path".into())
}

#[tauri::command]
fn demo_plot() -> ConvergencePlot {
    ConvergencePlot::vqe_demo()
}

#[derive(Deserialize)]
struct IdeAtomInput {
    symbol: String,
    x: f64,
    y: f64,
    z: f64,
}

#[tauri::command]
fn save_molecule_xyz(path: String, title: String, atoms: Vec<IdeAtomInput>) -> Result<String, String> {
    let base = Path::new(&path);
    let stem = base
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("mol");
    let out = base.with_file_name(format!("{stem}_edited.xyz"));
    let mut lines = vec![title.clone(), atoms.len().to_string()];
    for a in &atoms {
        lines.push(format!("{} {} {} {}", a.symbol.trim(), a.x, a.y, a.z));
    }
    let text = format!("{}\n", lines.join("\n"));
    fs::write(&out, &text).map_err(|e| format!("write {}: {e}", out.display()))?;
    out.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "invalid output path".into())
}

#[tauri::command]
fn run_python_snippet(source: String) -> RunResult {
    let output = match Command::new("python").args(["-c", &source]).output() {
        Ok(o) => o,
        Err(e) => {
            return RunResult {
                stdout: vec![],
                result: None,
                error: Some(format!("python: {e}")),
                backend: "python snippet".into(),
            };
        }
    };
    cli_output_to_result(output, "python snippet")
}

#[tauri::command]
fn export_molecule_mp4(
    path: String,
    camera: IdeCamera,
    frames: Option<u32>,
    style: Option<String>,
) -> Result<String, String> {
    let n = frames.unwrap_or(48).clamp(8, 120);
    let source = fs::read_to_string(&path).map_err(|e| format!("read {path}: {e}"))?;
    let geom = load_molecule_geometry(&path, &source)?;
    let render_style = style
        .as_deref()
        .map(MolRenderStyle::from_str_loose)
        .unwrap_or(MolRenderStyle::BallAndStick);
    let base = Path::new(&path);
    let frames_dir = base.with_extension("frames");
    if frames_dir.exists() {
        let _ = fs::remove_dir_all(&frames_dir);
    }
    fs::create_dir_all(&frames_dir).map_err(|e| format!("mkdir {}: {e}", frames_dir.display()))?;
    let base_yaw = camera.yaw;
    for i in 0..n {
        let mut cam: OrbitCamera = camera.clone().into();
        cam.yaw = base_yaw + (i as f32 / n as f32) * std::f32::consts::TAU;
        let png = render_molecule_png(&geom, &cam, render_style)?;
        fs::write(
            frames_dir.join(format!("frame_{i:04}.png")),
            png,
        )
        .map_err(|e| format!("write frame: {e}"))?;
    }
    let mp4 = base.with_extension("spin.mp4");
    let ffmpeg_ok = Command::new("ffmpeg")
        .args([
            "-y",
            "-framerate",
            "24",
            "-i",
            "frame_%04d.png",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(mp4.as_os_str())
        .current_dir(&frames_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ffmpeg_ok && mp4.is_file() {
        return Ok(format!("MP4: {}", mp4.display()));
    }
    Ok(format!(
        "Wrote {n} PNG frames to {} — install ffmpeg and re-export for MP4",
        frames_dir.display()
    ))
}

#[derive(Serialize)]
struct GjfCoordBlock {
    coordinate_type: String,
    lines: Vec<String>,
}

#[tauri::command]
fn gjf_get_coordinates(path: String) -> Result<GjfCoordBlock, String> {
    let source = fs::read_to_string(&path).map_err(|e| format!("read {path}: {e}"))?;
    let (lines, ty) = extract_gjf_coordinate_block(&source)?;
    Ok(GjfCoordBlock {
        coordinate_type: match ty {
            physlang_viz::CoordinateType::Cartesian => "cartesian".into(),
            physlang_viz::CoordinateType::ZMatrix => "z_matrix".into(),
        },
        lines,
    })
}

#[tauri::command]
fn gjf_set_coordinates(path: String, lines: Vec<String>) -> Result<(), String> {
    let source = fs::read_to_string(&path).map_err(|e| format!("read {path}: {e}"))?;
    let updated = replace_gjf_coordinate_block(&source, &lines)?;
    fs::write(&path, updated).map_err(|e| format!("write {path}: {e}"))
}

#[tauri::command]
fn export_field_mp4(
    path: String,
    index: usize,
    camera: IdeCamera,
    frames: Option<u32>,
    mode: Option<String>,
) -> Result<String, String> {
    let n = frames.unwrap_or(48).clamp(8, 120);
    let field = load_scalar_field(&path)?;
    let base = path.split('|').next().unwrap_or(&path);
    let base_path = Path::new(base);
    let frames_dir = base_path.with_extension("field.frames");
    if frames_dir.exists() {
        let _ = fs::remove_dir_all(&frames_dir);
    }
    fs::create_dir_all(&frames_dir).map_err(|e| format!("mkdir {}: {e}", frames_dir.display()))?;
    let use_iso = mode.as_deref() == Some("isosurface");
    let base_yaw = camera.yaw;
    for i in 0..n {
        let mut cam: OrbitCamera = camera.clone().into();
        cam.yaw = base_yaw + (i as f32 / n as f32) * std::f32::consts::TAU;
        let png = if use_iso {
            let iso = field_isovalue(&field, Some(0.35), 1.0);
            render_field_isosurface_png(&field, iso, &cam)?
        } else {
            render_field_slice_3d_png(&field, index, &cam)?
        };
        fs::write(
            frames_dir.join(format!("frame_{i:04}.png")),
            png,
        )
        .map_err(|e| format!("write frame: {e}"))?;
    }
    let mp4 = base_path.with_extension("field.spin.mp4");
    let ffmpeg_ok = Command::new("ffmpeg")
        .args([
            "-y",
            "-framerate",
            "24",
            "-i",
            "frame_%04d.png",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(mp4.as_os_str())
        .current_dir(&frames_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ffmpeg_ok && mp4.is_file() {
        return Ok(format!("MP4: {}", mp4.display()));
    }
    Ok(format!(
        "Wrote {n} field frames to {} — install ffmpeg for MP4",
        frames_dir.display()
    ))
}

#[tauri::command]
fn run_shell_command(cwd: Option<String>, command: String) -> RunResult {
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", &command]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", &command]);
        c
    };
    if let Some(dir) = cwd.filter(|d| !d.is_empty()) {
        cmd.current_dir(dir);
    }
    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            return RunResult {
                stdout: vec![],
                result: None,
                error: Some(format!("shell: {e}")),
                backend: "shell".into(),
            };
        }
    };
    cli_output_to_result(output, "shell")
}

#[tauri::command]
fn chem_list_backends() -> Vec<ChemBackendInfo> {
    list_chem_backends()
}

#[tauri::command]
fn chem_submit_gaussian(path: String) -> Result<ChemJobResult, String> {
    chem_jobs::submit_gaussian_job(&path)
}

#[tauri::command]
fn chem_submit_orca(path: String) -> Result<ChemJobResult, String> {
    chem_jobs::submit_orca_job(&path)
}

#[tauri::command]
fn chem_enqueue_gaussian(path: String) -> Result<ChemJobEnqueueResult, String> {
    enqueue_chem_job("gaussian", &path)
}

#[tauri::command]
fn chem_enqueue_orca(path: String) -> Result<ChemJobEnqueueResult, String> {
    enqueue_chem_job("orca", &path)
}

#[tauri::command]
fn load_field_file(path: String) -> Result<IdeFieldSlice, String> {
    if path.to_ascii_lowercase().ends_with(".cube") {
        return load_cube_file(path);
    }
    let source = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path))?;
    let field: ScalarField =
        serde_json::from_str(&source).map_err(|e| format!("field json: {e}"))?;
    field_to_ide(
        &path,
        field,
        Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("field")
            .to_string(),
    )
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
    let field = load_scalar_field(&path)?;
    let name = Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("field")
        .to_string();
    field_to_ide_at_index(&path, field, name, index)
}

#[tauri::command]
fn pick_molecule_atom_cmd(
    path: String,
    camera: IdeCamera,
    style: Option<String>,
    screen_x: f32,
    screen_y: f32,
) -> Result<Option<usize>, String> {
    let source = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path))?;
    let mol = load_molecule_geometry(&path, &source)?;
    let cam: OrbitCamera = camera.into();
    let render_style = MolRenderStyle::from_str_loose(style.as_deref().unwrap_or("ball_and_stick"));
    Ok(pick_molecule_atom(&mol, &cam, render_style, screen_x, screen_y))
}

#[tauri::command]
fn render_molecule_frame(
    path: String,
    camera: IdeCamera,
    style: Option<String>,
) -> Result<RenderFrameResult, String> {
    let source = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path))?;
    let mol = load_molecule_geometry(&path, &source)?;
    let cam: OrbitCamera = camera.into();
    let render_style = MolRenderStyle::from_str_loose(style.as_deref().unwrap_or("ball_and_stick"));
    let png = render_molecule_png(&mol, &cam, render_style)?;
    Ok(RenderFrameResult {
        png,
        backend: format!("wgpu-molecule-{render_style:?}"),
    })
}

fn field_isovalue(field: &ScalarField, level: Option<f64>, sign: f64) -> f64 {
    let t = level.unwrap_or(0.35).clamp(0.01, 0.99);
    let max_abs = field
        .values
        .iter()
        .map(|v| v.abs())
        .fold(0.0_f64, f64::max);
    let has_negative = field.values.iter().any(|&v| v < -1e-12);
    if has_negative && max_abs > 1e-20 {
        return sign.signum() * t * max_abs;
    }
    let (min, max) = field
        .values
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
            (lo.min(v), hi.max(v))
        });
    let span = (max - min).max(1e-12);
    min + t * span
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FieldSurfaceKind {
    Density,
    Mo(usize),
    Esp,
}

fn split_field_path(path: &str) -> (&str, FieldSurfaceKind) {
    if let Some((base, idx)) = path.rsplit_once("|mo:") {
        if let Ok(i) = idx.parse::<usize>() {
            return (base, FieldSurfaceKind::Mo(i));
        }
    }
    if path.ends_with("|esp") {
        return (&path[..path.len() - 4], FieldSurfaceKind::Esp);
    }
    if path.ends_with("|density") {
        return (&path[..path.len() - 9], FieldSurfaceKind::Density);
    }
    (path, FieldSurfaceKind::Density)
}

fn load_scalar_field(path: &str) -> Result<ScalarField, String> {
    if path == "demo://gaussian" {
        return Ok(demo_gaussian_field(32));
    }
    let (file_path, kind) = split_field_path(path);
    let lower = file_path.to_ascii_lowercase();
    if lower.ends_with(".cube") {
        let source = fs::read_to_string(file_path).map_err(|e| format!("read {file_path}: {e}"))?;
        let cube = parse_cube(&source)?;
        return Ok(cube_to_scalar_field(&cube));
    }
    if lower.ends_with(".fchk") {
        let source = fs::read_to_string(file_path).map_err(|e| format!("read {file_path}: {e}"))?;
        let fchk = parse_fchk(&source)?;
        return match kind {
            FieldSurfaceKind::Mo(mo_index) => fchk_mo_field(&fchk, mo_index, 32),
            FieldSurfaceKind::Esp => Ok(fchk_esp_field(&fchk, 32)),
            FieldSurfaceKind::Density => Ok(fchk_density_field(&fchk, 32)),
        };
    }
    let source = fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    serde_json::from_str(&source).map_err(|e| format!("field json: {e}"))
}

#[tauri::command]
fn render_field_frame(
    path: String,
    index: usize,
    camera: IdeCamera,
    mode: Option<String>,
    iso_level: Option<f64>,
    iso_sign: Option<f64>,
    iso_dual: Option<bool>,
) -> Result<RenderFrameResult, String> {
    let field = load_scalar_field(&path)?;
    let cam: OrbitCamera = camera.into();
    let use_iso = mode.as_deref() == Some("isosurface");
    let sign = iso_sign.unwrap_or(1.0);
    let (_, kind) = split_field_path(&path);
    let is_mo = matches!(kind, FieldSurfaceKind::Mo(_));
    let dual = iso_dual.unwrap_or(is_mo);
    let png = if use_iso {
        if dual {
            let level = iso_level.unwrap_or(0.35);
            render_field_mo_isosurface_png(&field, level, &cam)?
        } else {
            let iso = field_isovalue(&field, iso_level, sign);
            render_field_isosurface_png(&field, iso, &cam)?
        }
    } else if mode.as_deref() == Some("volume") {
        render_field_volume_png(&field, &cam)?
    } else {
        render_field_slice_3d_png(&field, index, &cam)?
    };
    Ok(RenderFrameResult {
        png,
        backend: if use_iso {
            if dual {
                "wgpu-mo-dual-isosurface".into()
            } else {
                "wgpu-isosurface".into()
            }
        } else if mode.as_deref() == Some("volume") {
            "wgpu-volume-gpu".into()
        } else {
            "wgpu-field-slice-3d".into()
        },
    })
}

fn load_fchk_source(path: &str) -> Result<(String, physlang_viz::FchkFile), String> {
    let (file_path, _) = split_field_path(path);
    let source = fs::read_to_string(file_path).map_err(|e| format!("read {file_path}: {e}"))?;
    let fchk = parse_fchk(&source)?;
    Ok((file_path.to_string(), fchk))
}

fn load_molecule_geometry(path: &str, source: &str) -> Result<physlang_viz::MoleculeGeometry, String> {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".log") {
        let log = parse_gaussian_log(source)?;
        return log
            .geometry
            .ok_or_else(|| "log: no standard orientation geometry".to_string());
    }
    if lower.ends_with(".gjf") || lower.ends_with(".com") {
        return Ok(parse_gjf(source)?.geometry);
    }
    if lower.ends_with(".fchk") {
        return Ok(parse_fchk(source)?.geometry);
    }
    parse_structure(source, Some(path))
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
        name: mol.name.clone(),
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
        chem: None,
    }
}

fn molecule_to_ide_chem(path: &str, job: physlang_viz::GaussianInput) -> IdeMolecule {
    let mut ide = molecule_to_ide(path, job.geometry);
    ide.name = job.title.clone();
    let fmt = if path.to_ascii_lowercase().ends_with(".com") {
        "com"
    } else {
        "gjf"
    };
    ide.chem = Some(IdeChemMeta {
        format: fmt.into(),
        title: job.title,
        route: Some(job.route),
        charge: Some(job.charge),
        multiplicity: Some(job.multiplicity),
        coordinate_type: Some(match job.coordinate_type {
            physlang_viz::CoordinateType::Cartesian => "cartesian".into(),
            physlang_viz::CoordinateType::ZMatrix => "z_matrix".into(),
        }),
        final_energy_hartree: None,
        scf_cycles: None,
        n_frequencies: None,
        has_density: None,
        has_mos: None,
    });
    ide
}

fn molecule_to_ide_log(path: &str, log: physlang_viz::GaussianLogResult) -> IdeMolecule {
    let geometry = log.geometry.clone().unwrap_or(physlang_viz::MoleculeGeometry {
        name: log.title.clone(),
        atoms: vec![],
        bonds: vec![],
    });
    let mut ide = molecule_to_ide(path, geometry);
    ide.name = log.title.clone();
    ide.chem = Some(IdeChemMeta {
        format: "log".into(),
        title: log.title,
        route: None,
        charge: None,
        multiplicity: None,
        coordinate_type: None,
        final_energy_hartree: log.final_energy_hartree,
        scf_cycles: Some(log.scf_energies_hartree.len()),
        n_frequencies: Some(log.n_frequencies),
        has_density: None,
        has_mos: None,
    });
    ide
}

fn molecule_to_ide_fchk(path: &str, fchk: physlang_viz::FchkFile) -> IdeMolecule {
    let mut ide = molecule_to_ide(path, fchk.geometry);
    ide.name = fchk.title.clone();
    ide.chem = Some(IdeChemMeta {
        format: "fchk".into(),
        title: fchk.title,
        route: None,
        charge: None,
        multiplicity: None,
        coordinate_type: None,
        final_energy_hartree: None,
        scf_cycles: None,
        n_frequencies: Some(fchk.vibrational_frequencies_cm1.len()),
        has_density: Some(fchk.has_density),
        has_mos: Some(fchk.has_mos),
    });
    ide
}

#[tauri::command]
fn parse_gjf_file(path: String) -> Result<IdeMolecule, String> {
    let source = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path))?;
    let job = parse_gjf(&source)?;
    Ok(molecule_to_ide_chem(&path, job))
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
                    Some("gjf") | Some("com") => "gjf",
                    Some("log") => "log",
                    Some("fchk") => "fchk",
                    Some("cube") => "cube",
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
fn format_phys_source(source: String) -> String {
    source
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

#[tauri::command]
fn complete_phys_prefix(
    source: String,
    prefix: String,
    project_root: Option<String>,
) -> Vec<IdeCompletion> {
    let mut items: Vec<IdeCompletion> = completions_for_prefix(&source, &prefix)
        .into_iter()
        .map(|c| IdeCompletion {
            label: c.label.clone(),
            detail: c.detail,
            insert_text: c.insert_text,
            kind: completion_kind_name(c.kind).to_string(),
        })
        .collect();
    if let Some(root) = project_root {
        if let Some(stdlib_dir) = find_stdlib_dir(Path::new(&root)) {
            let index = index_stdlib_dir(&stdlib_dir);
            let p = prefix.trim().to_lowercase();
            for (name, sym) in index {
                if !p.is_empty() && !name.to_lowercase().starts_with(&p) {
                    continue;
                }
                if items.iter().any(|i| i.label == name) {
                    continue;
                }
                let mut detail = match sym.file.file_name().and_then(|n| n.to_str()) {
                    Some(f) => format!("stdlib: {f}"),
                    None => "stdlib".into(),
                };
                if let Ok(src) = fs::read_to_string(&sym.file) {
                    let docs = doc_lines_before(&src, sym.line);
                    if let Some(last) = docs.last() {
                        detail = format!("{detail} — {last}");
                    }
                }
                items.push(IdeCompletion {
                    label: name.clone(),
                    detail: Some(detail),
                    insert_text: name,
                    kind: "function".into(),
                });
            }
        }
    }
    items
}

#[tauri::command]
fn find_phys_references(source: String, line: u32, column: u32) -> Vec<IdeLocation> {
    find_references_at(&source, line, column)
        .into_iter()
        .map(|l| IdeLocation {
            line: l.line,
            column: l.column,
            end_column: l.end_column,
            file: None,
        })
        .collect()
}

#[tauri::command]
fn rename_phys_symbol(source: String, line: u32, column: u32, new_name: String) -> Vec<IdeTextEdit> {
    rename_at(&source, line, column, &new_name)
        .into_iter()
        .map(|e| IdeTextEdit {
            line: e.line,
            column: e.column,
            end_column: e.end_column,
            new_text: e.new_text,
        })
        .collect()
}

#[tauri::command]
fn chem_job_status() -> ChemJobProgress {
    chem_job_progress()
}

#[tauri::command]
fn chem_job_last_result() -> Option<ChemJobResult> {
    last_chem_job_result()
}

#[tauri::command]
fn chem_job_cancel_cmd() -> Result<String, String> {
    chem_job_cancel()
}

#[tauri::command]
async fn chem_submit_gaussian_async(path: String) -> Result<ChemJobEnqueueResult, String> {
    tauri::async_runtime::spawn_blocking(move || enqueue_chem_job("gaussian", &path))
        .await
        .map_err(|e| format!("job thread: {e}"))?
}

#[tauri::command]
async fn chem_submit_orca_async(path: String) -> Result<ChemJobEnqueueResult, String> {
    tauri::async_runtime::spawn_blocking(move || enqueue_chem_job("orca", &path))
        .await
        .map_err(|e| format!("job thread: {e}"))?
}

#[tauri::command]
fn stdlib_reference_markdown(root: String) -> Result<String, String> {
    let stdlib = find_stdlib_dir(Path::new(&root))
        .ok_or_else(|| format!("stdlib/ not found under {root}"))?;
    Ok(generate_stdlib_markdown(&stdlib))
}

#[tauri::command]
fn goto_phys_definition(
    source: String,
    line: u32,
    column: u32,
    project_root: Option<String>,
) -> Option<IdeLocation> {
    let stdlib_index = project_root
        .as_ref()
        .and_then(|root| find_stdlib_dir(Path::new(root)))
        .map(|dir| index_stdlib_dir(&dir))
        .unwrap_or_default();
    definition_at_with_stdlib(&source, line, column, &stdlib_index).map(|loc| IdeLocation {
        line: loc.line,
        column: loc.column,
        end_column: loc.end_column,
        file: loc.file,
    })
}

#[tauri::command]
fn phys_code_actions(source: String, line: u32, column: u32) -> Vec<IdeCodeAction> {
    code_actions_at(&source, line, column)
        .into_iter()
        .map(|a| IdeCodeAction {
            title: a.title,
            edits: a
                .edits
                .into_iter()
                .map(|e| IdeTextEdit {
                    line: e.line,
                    column: e.column,
                    end_column: e.end_column,
                    new_text: e.new_text,
                })
                .collect(),
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
fn debug_eval_phys(expr: String) -> RunResult {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return RunResult {
            stdout: vec![],
            result: None,
            error: Some("empty expression".into()),
            backend: "debug-eval".into(),
        };
    }
    let source = format!("fn main() -> Int {{\n    return ({trimmed})\n}}\n");
    run_phys_source(source, None)
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
            write_binary_file,
            list_phys_files,
            list_package_catalog,
            list_installed_packages,
            install_package,
            debug_eval_phys,
            parse_molecule_file,
            parse_gaussian_log_file,
            load_cube_file,
            load_fchk_density_file,
            load_fchk_mo_file,
            load_fchk_esp_file,
            fchk_list_mos,
            parse_fchk_file,
            list_vibration_modes,
            vibration_frame,
            parse_gjf_file,
            parse_molecule_xyz,
            load_field_file,
            demo_scalar_field,
            field_slice_at,
            render_molecule_frame,
            pick_molecule_atom_cmd,
            render_field_frame,
            export_fchk_density_cube,
            export_field_vtk,
            demo_plot,
            save_molecule_xyz,
            run_python_snippet,
            export_molecule_mp4,
            export_field_mp4,
            gjf_get_coordinates,
            gjf_set_coordinates,
            run_shell_command,
            chem_list_backends,
            chem_submit_gaussian,
            chem_submit_orca,
            chem_submit_gaussian_async,
            chem_submit_orca_async,
            chem_job_status,
            chem_job_last_result,
            chem_job_cancel_cmd,
            goto_phys_definition,
            stdlib_reference_markdown,
            phys_code_actions,
            check_phys_source,
            format_phys_source,
            complete_phys_prefix,
            find_phys_references,
            rename_phys_symbol,
            hover_phys_source,
            run_phys_source,
            run_phys_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Inertia IDE");
}
