import "./styles.css";
import * as monaco from "monaco-editor";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { FieldViewer, type FieldSliceData } from "./field-viewer";
import {
  applyDiagnostics,
  fetchDiagnostics,
  registerLspProviders,
} from "./lsp";
import { MoleculeViewer, type MoleculeData, type MeasureMode } from "./molecule-viewer";
import {
  PHYSLANG_LANGUAGE_ID,
  registerPhyslangLanguage,
} from "./physlang-monarch";

self.MonacoEnvironment = {
  getWorker() {
    return new editorWorker();
  },
};

registerPhyslangLanguage(monaco);

interface RunResult {
  stdout: string[];
  result?: string;
  error?: string;
  backend: string;
}

interface ProjectFile {
  path: string;
  name: string;
  kind: string;
}

interface FchkMoInfo {
  index: number;
  label: string;
  energy_hartree?: number;
  occupied: boolean;
}

type ViewerTab = "molecule" | "field";

let editor: monaco.editor.IStandaloneCodeEditor;
let moleculeViewer: MoleculeViewer | null = null;
let fieldViewer: FieldViewer | null = null;
let currentPath: string | null = null;
let currentMoleculePath: string | null = null;
let currentFieldPath: string | null = null;
let currentFieldDepth = 0;
let projectRoot: string | null = null;
let dirty = false;
let scheduleDiagnostics: (() => void) | null = null;
let vibPath: string | null = null;
let vibAnimHandle: ReturnType<typeof setInterval> | null = null;
let vibPhase = 0;

interface VibrationModeInfo {
  index: number;
  frequency_cm1: number;
}

interface ChemJobProgress {
  running: boolean;
  backend: string;
  input_path: string;
  message: string;
  stderr_tail: string;
  queue: { backend: string; input_path: string }[];
  last_completed_path?: string;
}

interface ChemJobEnqueueResult {
  queued: boolean;
  queue_position: number;
  message: string;
}

interface ChemJobResult {
  success: boolean;
  command: string;
  log_path?: string;
  stdout: string;
  stderr: string;
  message: string;
}

interface ChemBackendInfo {
  id: string;
  executable: string;
}

function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  className?: string,
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  if (className) node.className = className;
  return node;
}

function setOutput(text: string, kind: "plain" | "error" | "success" = "plain") {
  const output = document.getElementById("output")!;
  output.innerHTML = "";
  const span = document.createElement("span");
  if (kind !== "plain") span.className = kind;
  span.textContent = text;
  output.appendChild(span);
}

function appendOutput(text: string) {
  const output = document.getElementById("output")!;
  output.textContent += text;
}

function updateFileLabel() {
  const label = document.getElementById("file-label")!;
  if (currentPath) {
    const suffix = dirty ? " •" : "";
    label.textContent = `${currentPath}${suffix}`;
    return;
  }
  label.textContent =
    currentMoleculePath ?? currentFieldPath ?? "Untitled.phys";
}

function setViewerVisible(visible: boolean) {
  const pane = document.getElementById("viewer-pane")!;
  pane.classList.toggle("hidden", !visible);
  moleculeViewer?.resize();
  fieldViewer?.resize();
}

function switchViewerTab(tab: ViewerTab) {
  document.getElementById("mol-view")?.classList.toggle("hidden", tab !== "molecule");
  document.getElementById("field-view")?.classList.toggle("hidden", tab !== "field");
  document.getElementById("tab-molecule")?.classList.toggle("active", tab === "molecule");
  document.getElementById("tab-field")?.classList.toggle("active", tab === "field");
  moleculeViewer?.resize();
  fieldViewer?.resize();
}

async function loadMolecule(path: string, editInEditor = false) {
  try {
    if (editInEditor) {
      const source = await invoke<string>("read_text_file", { path });
      currentPath = path;
      dirty = false;
      editor.setValue(source);
      const model = editor.getModel();
      if (model) {
        monaco.editor.setModelLanguage(model, "plaintext");
      }
      updateFileLabel();
    }
    const mol = await invoke<MoleculeData>("parse_molecule_file", { path });
    currentMoleculePath = path;
    currentFieldPath = null;
    setMeasureMode("off");
    moleculeViewer?.load(mol);
    updateChemBar(mol);
    void setupVibrationControls(path);
    switchViewerTab("molecule");
    setViewerVisible(true);
    highlightTreeSelection(path);
    const chemNote = mol.chem
      ? mol.chem.final_energy_hartree != null
        ? ` | E = ${mol.chem.final_energy_hartree.toFixed(6)} Ha`
        : mol.chem.route
          ? ` | ${mol.chem.format.toUpperCase()}`
          : ""
      : "";
    setOutput(`Loaded molecule: ${mol.name} (${mol.atoms.length} atoms)${chemNote}`, "success");
  } catch (err) {
    setOutput(String(err), "error");
  }
}

function updateChemBar(mol: MoleculeData) {
  const bar = document.getElementById("chem-bar")!;
  if (!mol.chem) {
    bar.classList.add("hidden");
    bar.textContent = "";
    return;
  }
  bar.classList.remove("hidden");
  const c = mol.chem;
  const parts: string[] = [c.format.toUpperCase(), c.title];
  if (c.route) parts.push(c.route);
  if (c.charge != null && c.multiplicity != null) {
    parts.push(`charge ${c.charge} mult ${c.multiplicity}`);
  }
  if (c.coordinate_type) parts.push(c.coordinate_type);
  if (c.final_energy_hartree != null) {
    parts.push(`E = ${c.final_energy_hartree.toFixed(6)} Ha`);
  }
  if (c.scf_cycles != null && c.scf_cycles > 0) {
    parts.push(`${c.scf_cycles} SCF cycles`);
  }
  if (c.n_frequencies != null && c.n_frequencies > 0) {
    parts.push(`${c.n_frequencies} freq`);
  }
  if (c.has_density) parts.push("density");
  if (c.has_mos) parts.push("MOs");
  bar.textContent = parts.join(" · ");
  bar.title = c.title;
  updateSurfaceBar(mol);
}

function updateSurfaceBar(mol: MoleculeData) {
  const bar = document.getElementById("surface-bar")!;
  const show = mol.chem?.format === "fchk" || mol.chem?.has_density;
  bar.classList.toggle("hidden", !show);
  const moSelect = document.getElementById("mo-select") as HTMLSelectElement | null;
  const moLabel = document.getElementById("mo-label");
  const homoBtn = document.getElementById("surface-homo");
  const lumoBtn = document.getElementById("surface-lumo");
  const hasMos = mol.chem?.has_mos === true;
  moSelect?.classList.toggle("hidden", !hasMos);
  moLabel?.classList.toggle("hidden", !hasMos);
  homoBtn?.classList.toggle("hidden", !hasMos);
  lumoBtn?.classList.toggle("hidden", !hasMos);
  if (hasMos && currentMoleculePath) {
    void refreshMoSelect(currentMoleculePath);
  } else if (moSelect) {
    moSelect.innerHTML = "";
  }
}

async function refreshMoSelect(path: string) {
  const moSelect = document.getElementById("mo-select") as HTMLSelectElement | null;
  if (!moSelect) return;
  try {
    const mos = await invoke<FchkMoInfo[]>("fchk_list_mos", { path });
    moSelect.innerHTML = "";
    for (const mo of mos) {
      const opt = document.createElement("option");
      opt.value = String(mo.index);
      const energy =
        mo.energy_hartree != null ? ` (${mo.energy_hartree.toFixed(4)} Ha)` : "";
      opt.textContent = `${mo.label}${energy}`;
      moSelect.appendChild(opt);
    }
  } catch {
    moSelect.innerHTML = "";
  }
}

async function loadFchkMo(path: string, moIndex: number) {
  try {
    const slice = await invoke<FieldSliceData>("load_fchk_mo_file", {
      path,
      moIndex,
    });
    await applyFieldSlice(slice, true);
  } catch (err) {
    setOutput(String(err), "error");
  }
}

async function loadFchkEsp(path: string) {
  try {
    const slice = await invoke<FieldSliceData>("load_fchk_esp_file", { path });
    await applyFieldSlice(slice, true);
  } catch (err) {
    setOutput(String(err), "error");
  }
}

async function loadFchkDensity(path: string) {
  try {
    const slice = await invoke<FieldSliceData>("load_fchk_density_file", { path });
    await applyFieldSlice(slice, true);
  } catch (err) {
    setOutput(String(err), "error");
  }
}

async function exportFchkCube(path: string) {
  try {
    const out = await invoke<string>("export_fchk_density_cube", { path });
    setOutput(`Exported density cube: ${out}`, "success");
    await loadField(out, true);
  } catch (err) {
    setOutput(String(err), "error");
  }
}

function setJobStatus(text: string, running = false) {
  const el = document.getElementById("job-status");
  if (!el) return;
  el.textContent = text;
  el.classList.toggle("running", running);
}

function updateJobPanel(progress: ChemJobProgress) {
  const panel = document.getElementById("job-panel-body");
  if (!panel) return;
  const lines: string[] = [];
  if (progress.running) {
    lines.push(`▶ ${progress.backend.toUpperCase()}: ${basename(progress.input_path)}`);
    lines.push(progress.message);
  } else if (progress.message) {
    lines.push(progress.message);
  }
  if (progress.queue.length > 0) {
    lines.push("Queue:");
    for (const q of progress.queue) {
      lines.push(`  · ${q.backend} — ${basename(q.input_path)}`);
    }
  }
  if (progress.stderr_tail.trim()) {
    lines.push("--- stderr (live) ---");
    lines.push(progress.stderr_tail.trim());
  }
  panel.textContent = lines.join("\n") || "No jobs";
  const cancelBtn = document.getElementById("job-cancel-btn") as HTMLButtonElement | null;
  if (cancelBtn) cancelBtn.disabled = !progress.running;
}

function basename(path: string): string {
  const parts = path.replace(/\\/g, "/").split("/");
  return parts[parts.length - 1] ?? path;
}

function sleep(ms: number) {
  return new Promise((r) => setTimeout(r, ms));
}

async function waitForChemJob(path: string) {
  while (true) {
    const p = await invoke<ChemJobProgress>("chem_job_status");
    updateJobPanel(p);
    setJobStatus(
      p.running
        ? `${p.backend}: ${p.message}`
        : p.queue.length
          ? `queued (${p.queue.length})`
          : "idle",
      p.running,
    );
    if (p.stderr_tail.trim()) {
      setOutput(
        `${p.message}\n--- stderr (live) ---\n${p.stderr_tail.trim()}`,
        "plain",
      );
    }
    const stillPending =
      (p.running && p.input_path === path) ||
      p.queue.some((q) => q.input_path === path);
    if (p.last_completed_path === path && !stillPending) break;
    await sleep(350);
  }
}

async function cancelChemJob() {
  try {
    await invoke("chem_job_cancel_cmd");
    setOutput("Cancel requested…", "plain");
  } catch (err) {
    setOutput(String(err), "error");
  }
}

async function runChemJob(backend: "gaussian" | "orca") {
  const path = currentPath ?? currentMoleculePath;
  if (!path?.toLowerCase().match(/\.(gjf|com)$/)) {
    setOutput("Open a .gjf or .com file first", "error");
    return;
  }
  const label = backend === "gaussian" ? "Gaussian" : "ORCA";
  try {
    const backends = await invoke<ChemBackendInfo[]>("chem_list_backends");
    if (!backends.some((b) => b.id === backend)) {
      setOutput(
        `${label} not found. Set ${backend === "gaussian" ? "GAUSSIAN_EXE" : "ORCA_EXE"} or add to PATH.\n` +
          (backends.length ? `Found: ${backends.map((b) => b.id).join(", ")}` : "No chem backends detected."),
        "error",
      );
      setJobStatus("idle");
      return;
    }

    const cmd =
      backend === "gaussian" ? "chem_submit_gaussian_async" : "chem_submit_orca_async";
    const enq = await invoke<ChemJobEnqueueResult>(cmd, { path });
    setOutput(`${label}: ${enq.message}\n`, "plain");
    setJobStatus(enq.queued ? `${label} queued` : `${label} running…`, true);
    document.getElementById("job-panel")?.classList.remove("hidden");

    await waitForChemJob(path);

    const result = await invoke<ChemJobResult | null>("chem_job_last_result");
    if (!result) {
      setJobStatus("idle");
      return;
    }

    const stderrBlock = result.stderr.trim()
      ? `\n--- stderr ---\n${result.stderr.trim()}`
      : "";
    const stdoutBlock = result.stdout.trim()
      ? `\n--- stdout ---\n${result.stdout.trim()}`
      : "";
    const lines = [result.message, `cmd: ${result.command}`, stdoutBlock, stderrBlock]
      .filter(Boolean)
      .join("\n");
    setOutput(lines, result.success ? "success" : "error");
    setJobStatus(result.success ? "done" : "failed");
    updateJobPanel(await invoke<ChemJobProgress>("chem_job_status"));

    if (result.log_path && result.success) {
      const lower = result.log_path.toLowerCase();
      if (lower.endsWith(".log")) {
        await loadMolecule(result.log_path, true);
      } else if (lower.endsWith(".out")) {
        await loadFile(result.log_path);
      }
    }
  } catch (err) {
    setOutput(String(err), "error");
    setJobStatus("failed");
  }
}

async function runGaussianJob() {
  await runChemJob("gaussian");
}

async function runOrcaJob() {
  await runChemJob("orca");
}

function setMeasureMode(mode: MeasureMode) {
  moleculeViewer?.setMeasureMode(mode);
  const bar = document.getElementById("measure-bar")!;
  bar.querySelectorAll("button[data-measure]").forEach((btn) => {
    btn.classList.toggle("active", (btn as HTMLButtonElement).dataset.measure === mode);
  });
  const label = document.getElementById("measure-label")!;
  label.textContent = moleculeViewer?.getMeasureText() ?? "";
}

function refreshMeasureLabel() {
  const label = document.getElementById("measure-label")!;
  label.textContent = moleculeViewer?.getMeasureText() ?? "";
}

function stopVibrationAnim() {
  if (vibAnimHandle !== null) {
    clearInterval(vibAnimHandle);
    vibAnimHandle = null;
  }
}

async function setupVibrationControls(path: string) {
  stopVibrationAnim();
  vibPath = null;
  vibPhase = 0;
  const bar = document.getElementById("vib-bar")!;
  bar.classList.add("hidden");
  const lower = path.toLowerCase();
  if (!lower.endsWith(".log") && !lower.endsWith(".fchk")) return;
  try {
    const info = await invoke<{ path: string; modes: VibrationModeInfo[] }>(
      "list_vibration_modes",
      { path },
    );
    if (info.modes.length === 0) return;
    vibPath = path;
    bar.classList.remove("hidden");
    const select = document.getElementById("vib-mode") as HTMLSelectElement;
    select.innerHTML = "";
    for (const m of info.modes) {
      const opt = document.createElement("option");
      opt.value = String(m.index);
      opt.textContent = `Mode ${m.index + 1} (${m.frequency_cm1.toFixed(1)} cm⁻¹)`;
      select.appendChild(opt);
    }
    await showVibrationFrame(0, 0);
  } catch {
    /* no modes */
  }
}

async function showVibrationFrame(modeIndex: number, phase: number) {
  if (!vibPath) return;
  const mol = await invoke<MoleculeData>("vibration_frame", {
    path: vibPath,
    modeIndex,
    phase,
  });
  moleculeViewer?.loadDirect(mol);
}

async function onVibModeChange() {
  vibPhase = 0;
  const select = document.getElementById("vib-mode") as HTMLSelectElement;
  await showVibrationFrame(Number(select.value), 0);
}

function toggleVibrationPlay() {
  const btn = document.getElementById("vib-play") as HTMLButtonElement;
  if (vibAnimHandle) {
    stopVibrationAnim();
    btn.textContent = "▶ Animate";
    return;
  }
  btn.textContent = "⏸ Stop";
  vibAnimHandle = setInterval(() => {
    vibPhase = (vibPhase + 0.04) % 1;
    const select = document.getElementById("vib-mode") as HTMLSelectElement;
    void showVibrationFrame(Number(select.value), vibPhase);
  }, 50);
}

async function applyFieldSlice(slice: FieldSliceData, preferIso = false) {
  currentFieldPath = slice.path;
  currentFieldDepth = slice.depth;
  currentMoleculePath = null;
  fieldViewer?.load(slice, slice.depth - 1);
  if (preferIso) {
    fieldViewer?.setViewMode("isosurface");
    document.querySelectorAll(".slice-bar .field-mode-btn").forEach((btn, i) => {
      btn.classList.toggle("active", i === 2);
    });
    document.getElementById("iso-row")?.classList.remove("hidden");
  }
  const slider = document.getElementById("slice-slider") as HTMLInputElement;
  if (slider) {
    slider.max = String(slice.depth - 1);
    slider.value = String(slice.index);
    document.getElementById("slice-label")!.textContent = `Z = ${slice.index}`;
  }
  switchViewerTab("field");
  setViewerVisible(true);
  const wgpuNote = slice.wgpu_png?.length
    ? " (wgpu validated)"
    : "";
  setOutput(
    `Loaded field: ${slice.name} [${slice.width}×${slice.height} slice]${wgpuNote} — use 3D/Iso toggles`,
    "success",
  );
}

async function loadField(path: string, isCube = false) {
  try {
    const slice = isCube
      ? await invoke<FieldSliceData>("load_cube_file", { path })
      : await invoke<FieldSliceData>("load_field_file", { path });
    await applyFieldSlice(slice, isCube);
    highlightTreeSelection(path);
  } catch (err) {
    setOutput(String(err), "error");
  }
}

async function loadDemoField() {
  try {
    const slice = await invoke<FieldSliceData>("demo_scalar_field", {
      resolution: 32,
    });
    await applyFieldSlice(slice);
  } catch (err) {
    setOutput(String(err), "error");
  }
}

async function onSliceChange(index: number) {
  if (!currentFieldPath || currentFieldDepth === 0) return;
  try {
    const slice = await invoke<FieldSliceData>("field_slice_at", {
      path: currentFieldPath,
      index,
    });
    fieldViewer?.updateSlice(slice);
    document.getElementById("slice-label")!.textContent = `Z = ${index}`;
  } catch (err) {
    setOutput(String(err), "error");
  }
}

async function loadFile(path: string, content?: string) {
  const source =
    content ?? (await invoke<string>("read_text_file", { path }));
  currentPath = path;
  dirty = false;
  editor.setValue(source);
  const model = editor.getModel();
  if (model) {
    monaco.editor.setModelLanguage(model, PHYSLANG_LANGUAGE_ID);
  }
  updateFileLabel();
  highlightTreeSelection(path);
  scheduleDiagnostics?.();
}

async function openProjectEntry(file: ProjectFile) {
  if (file.kind === "gjf" || file.kind === "log" || file.kind === "fchk") {
    await loadMolecule(file.path, true);
  } else if (file.kind === "xyz" || file.kind === "pdb") {
    await loadMolecule(file.path);
  } else if (file.kind === "field" || file.kind === "cube") {
    await loadField(file.path, file.kind === "cube");
  } else {
    await loadFile(file.path);
  }
}

function fileTreeIcon(file: ProjectFile): string {
  switch (file.kind) {
    case "gjf":
      return `⚛ ${file.name}`;
    case "log":
      return `📋 ${file.name}`;
    case "fchk":
      return `📦 ${file.name}`;
    case "cube":
      return `☁ ${file.name}`;
    case "xyz":
    case "pdb":
      return `⬡ ${file.name}`;
    case "field":
      return `▣ ${file.name}`;
    default:
      return file.name;
  }
}

async function refreshProjectTree() {
  const tree = document.getElementById("file-tree")!;
  tree.innerHTML = "";
  if (!projectRoot) {
    const hint = el("p");
    hint.textContent = "Open a folder — .phys, .gjf, .log, .cube, .xyz, .pdb, .field.json";
    hint.style.padding = "8px 12px";
    hint.style.color = "#888";
    tree.appendChild(hint);
    return;
  }

  const files = await invoke<ProjectFile[]>("list_phys_files", {
    root: projectRoot,
  });

  for (const file of files) {
    const btn = el("button");
    btn.textContent = fileTreeIcon(file);
    btn.title = file.path;
    btn.dataset.path = file.path;
    if (
      file.path === currentPath ||
      file.path === currentMoleculePath ||
      file.path === currentFieldPath
    ) {
      btn.classList.add("active");
    }
    btn.addEventListener("click", () => openProjectEntry(file));
    tree.appendChild(btn);
  }
}

function highlightTreeSelection(path: string) {
  document.querySelectorAll("#file-tree button").forEach((node) => {
    const btn = node as HTMLButtonElement;
    btn.classList.toggle("active", btn.dataset.path === path);
  });
}

async function openFileDialog() {
  const selected = await open({
    multiple: false,
    filters: [{ name: "PhysicsLang", extensions: ["phys"] }],
  });
  if (typeof selected === "string") await loadFile(selected);
}

async function openMoleculeDialog() {
  const selected = await open({
    multiple: false,
    filters: [
      { name: "Molecules / Gaussian", extensions: ["xyz", "pdb", "gjf", "com", "log"] },
    ],
  });
  if (typeof selected === "string") {
    const lower = selected.toLowerCase();
    await loadMolecule(selected, lower.endsWith(".gjf") || lower.endsWith(".com") || lower.endsWith(".log"));
  }
}

async function openLanguageDocs() {
  if (!projectRoot) {
    setOutput("Open the Inertia repo folder first (Open Folder), then click Docs again.", "error");
    return;
  }
  const sep = projectRoot.includes("\\") ? "\\" : "/";
  const docPath = `${projectRoot}${sep}docs${sep}language-reference.md`;
  try {
    const source = await invoke<string>("read_text_file", { path: docPath });
    currentPath = docPath;
    currentMoleculePath = null;
    currentFieldPath = null;
    dirty = false;
    editor.setValue(source);
    const model = editor.getModel();
    if (model) {
      monaco.editor.setModelLanguage(model, "markdown");
    }
    updateFileLabel();
    setOutput("PhysicsLang language reference — see also docs/quickstart.md", "success");
    scheduleDiagnostics?.();
  } catch (err) {
    setOutput(`Could not load ${docPath}: ${err}`, "error");
  }
}

async function openFolderDialog() {
  const selected = await open({ directory: true, multiple: false });
  if (typeof selected === "string") {
    projectRoot = selected;
    await refreshProjectTree();
    const files = await invoke<ProjectFile[]>("list_phys_files", {
      root: projectRoot,
    });
    if (!currentPath) {
      const firstPhys = files.find((f) => f.kind === "phys");
      if (firstPhys) await loadFile(firstPhys.path);
    }
  }
}

async function saveCurrentFile() {
  if (!currentPath) {
    const selected = await save({
      filters: [{ name: "PhysicsLang", extensions: ["phys"] }],
      defaultPath: "untitled.phys",
    });
    if (!selected) return;
    currentPath = selected;
  }
  const source = editor.getValue();
  await invoke("write_text_file", { path: currentPath, content: source });
  dirty = false;
  updateFileLabel();
  appendOutput(`\nSaved ${currentPath}`);
}

async function checkCurrent() {
  const source = editor.getValue();
  setOutput("Checking…");
  try {
    const diags = await fetchDiagnostics(source);
    const model = editor.getModel();
    if (model) applyDiagnostics(model, diags);
    if (diags.length === 0) {
      setOutput("No diagnostics — type check passed.", "success");
    } else {
      setOutput(
        diags.map((d) => `${d.line}:${d.column}: ${d.message}`).join("\n"),
        "error",
      );
    }
  } catch (err) {
    setOutput(String(err), "error");
  }
}

async function runCurrent() {
  const source = editor.getValue();
  setOutput("Running…\n");
  try {
    const result: RunResult = currentPath
      ? await invoke("run_phys_file", {
          path: currentPath,
          entry: null,
          projectRoot,
        })
      : await invoke("run_phys_source", { source, entry: null });

    const lines = [
      `backend: ${result.backend}`,
      ...result.stdout,
      result.result ? `=> ${result.result}` : "",
    ]
      .filter(Boolean)
      .join("\n");

    setOutput(
      result.error ? `${lines}\n\n${result.error}` : lines,
      result.error ? "error" : "success",
    );
  } catch (err) {
    setOutput(String(err), "error");
  }
}

function buildShell() {
  const app = document.getElementById("app")!;

  const toolbar = el("div", "toolbar");
  const mkBtn = (label: string, onClick: () => void, secondary = false) => {
    const btn = el("button");
    btn.textContent = label;
    if (secondary) btn.classList.add("secondary");
    btn.addEventListener("click", onClick);
    return btn;
  };

  toolbar.append(
    mkBtn("Open Folder", () => void openFolderDialog(), true),
    mkBtn("Open File", () => void openFileDialog(), true),
    mkBtn("Open Mol", () => void openMoleculeDialog(), true),
    mkBtn("Demo Field", () => void loadDemoField(), true),
    mkBtn("Docs", () => void openLanguageDocs(), true),
    mkBtn("Run G16", () => void runGaussianJob(), true),
    mkBtn("Run ORCA", () => void runOrcaJob(), true),
    mkBtn("Save", () => void saveCurrentFile(), true),
    mkBtn("Check", () => void checkCurrent(), true),
    mkBtn("Run", () => void runCurrent()),
  );
  const spacer = el("div", "spacer");
  const fileLabel = el("span", "file-label");
  fileLabel.id = "file-label";
  const jobStatus = el("span", "job-status");
  jobStatus.id = "job-status";
  jobStatus.textContent = "idle";
  toolbar.append(spacer, jobStatus, fileLabel);
  app.appendChild(toolbar);

  const workspace = el("div", "workspace");

  const sidebar = el("aside", "sidebar");
  const sidebarTitle = el("h2");
  sidebarTitle.textContent = "Explorer";
  const fileTree = el("div", "file-tree");
  fileTree.id = "file-tree";
  sidebar.append(sidebarTitle, fileTree);

  const center = el("div", "center-column");
  const editorPane = el("div", "editor-pane");
  const editorDiv = el("div");
  editorDiv.id = "editor";
  editorPane.appendChild(editorDiv);
  center.appendChild(editorPane);

  const viewerPane = el("aside", "viewer-pane hidden");
  viewerPane.id = "viewer-pane";

  const viewerTabs = el("div", "viewer-tabs");
  const tabMol = el("button", "viewer-tab active");
  tabMol.id = "tab-molecule";
  tabMol.textContent = "Molecule";
  tabMol.addEventListener("click", () => switchViewerTab("molecule"));
  const tabField = el("button", "viewer-tab");
  tabField.id = "tab-field";
  tabField.textContent = "Field";
  tabField.addEventListener("click", () => switchViewerTab("field"));
  viewerTabs.append(tabMol, tabField);

  const molView = el("div", "viewer-panel");
  molView.id = "mol-view";
  const molWrap = el("div", "viewer-canvas-wrap");
  const molCanvas = document.createElement("canvas");
  molCanvas.id = "molecule-canvas";
  molWrap.appendChild(molCanvas);
  molView.appendChild(molWrap);
  const chemBar = el("div", "chem-bar hidden");
  chemBar.id = "chem-bar";
  molView.appendChild(chemBar);
  const molStyleBar = el("div", "slice-bar");
  const mkMolStyle = (label: string, style: import("./wgpu-viewer").MolRenderStyle, active = false) => {
    const btn = el("button", "field-mode-btn");
    if (active) btn.classList.add("active");
    btn.textContent = label;
    btn.addEventListener("click", () => {
      moleculeViewer?.setRenderStyle(style);
      molStyleBar.querySelectorAll("button").forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
    });
    return btn;
  };
  molStyleBar.append(
    mkMolStyle("Ball+Stick", "ball_and_stick", true),
    mkMolStyle("Wire", "wireframe"),
    mkMolStyle("Space", "space_fill"),
    mkMolStyle("Stick", "stick"),
  );
  const fitBtn = el("button", "field-mode-btn");
  fitBtn.textContent = "Fit";
  fitBtn.title = "Reset camera";
  fitBtn.addEventListener("click", () => moleculeViewer?.resetView());
  molStyleBar.appendChild(fitBtn);
  molView.appendChild(molStyleBar);
  const surfaceBar = el("div", "slice-bar hidden");
  surfaceBar.id = "surface-bar";
  const surfaceLabel = el("span");
  surfaceLabel.textContent = "Surfaces";
  const densityBtn = el("button", "field-mode-btn");
  densityBtn.textContent = "Density";
  densityBtn.title = "SCF GTO density grid (promolecule if basis missing)";
  densityBtn.addEventListener("click", () => {
    if (currentMoleculePath) void loadFchkDensity(currentMoleculePath);
  });
  const homoBtn = el("button", "field-mode-btn");
  homoBtn.id = "surface-homo";
  homoBtn.textContent = "HOMO";
  homoBtn.classList.add("hidden");
  homoBtn.title = "Highest occupied MO isosurface";
  homoBtn.addEventListener("click", async () => {
    if (!currentMoleculePath) return;
    try {
      const mos = await invoke<FchkMoInfo[]>("fchk_list_mos", {
        path: currentMoleculePath,
      });
      const homo = mos.find((m) => m.label.startsWith("HOMO"));
      if (homo) void loadFchkMo(currentMoleculePath, homo.index);
    } catch (err) {
      setOutput(String(err), "error");
    }
  });
  const lumoBtn = el("button", "field-mode-btn");
  lumoBtn.id = "surface-lumo";
  lumoBtn.textContent = "LUMO";
  lumoBtn.classList.add("hidden");
  lumoBtn.title = "Lowest unoccupied MO isosurface";
  lumoBtn.addEventListener("click", async () => {
    if (!currentMoleculePath) return;
    try {
      const mos = await invoke<FchkMoInfo[]>("fchk_list_mos", {
        path: currentMoleculePath,
      });
      const lumo = mos.find((m) => m.label.startsWith("LUMO"));
      if (lumo) void loadFchkMo(currentMoleculePath, lumo.index);
    } catch (err) {
      setOutput(String(err), "error");
    }
  });
  const moLabel = el("span");
  moLabel.id = "mo-label";
  moLabel.textContent = "MO";
  moLabel.classList.add("hidden");
  const moSelect = document.createElement("select");
  moSelect.id = "mo-select";
  moSelect.classList.add("hidden");
  moSelect.title = "Select molecular orbital";
  moSelect.addEventListener("change", () => {
    if (currentMoleculePath) {
      void loadFchkMo(currentMoleculePath, Number(moSelect.value));
    }
  });
  const espBtn = el("button", "field-mode-btn");
  espBtn.id = "surface-esp";
  espBtn.textContent = "ESP";
  espBtn.title = "Electrostatic potential (quantum Hartree from ρ, or classical monopole)";
  espBtn.addEventListener("click", () => {
    if (currentMoleculePath) void loadFchkEsp(currentMoleculePath);
  });
  const exportCubeBtn = el("button", "field-mode-btn");
  exportCubeBtn.textContent = "Export .cube";
  exportCubeBtn.title = "Write density grid to .density.cube";
  exportCubeBtn.addEventListener("click", () => {
    if (currentMoleculePath) void exportFchkCube(currentMoleculePath);
  });
  surfaceBar.append(
    surfaceLabel,
    densityBtn,
    homoBtn,
    lumoBtn,
    moLabel,
    moSelect,
    espBtn,
    exportCubeBtn,
  );
  molView.appendChild(surfaceBar);
  const measureBar = el("div", "slice-bar");
  measureBar.id = "measure-bar";
  const measureTitle = el("span");
  measureTitle.textContent = "Measure";
  const mkMeasure = (label: string, mode: MeasureMode) => {
    const btn = el("button", "field-mode-btn");
    btn.textContent = label;
    btn.dataset.measure = mode;
    btn.addEventListener("click", () => {
      setMeasureMode(mode);
      refreshMeasureLabel();
    });
    return btn;
  };
  const measureClear = el("button", "field-mode-btn");
  measureClear.textContent = "Clear";
  measureClear.addEventListener("click", () => {
    moleculeViewer?.clearMeasure();
    refreshMeasureLabel();
  });
  const measureLabel = el("span");
  measureLabel.id = "measure-label";
  measureLabel.style.flex = "1";
  measureLabel.style.fontSize = "11px";
  measureLabel.style.color = "#ffd700";
  measureBar.append(
    measureTitle,
    mkMeasure("Dist", "distance"),
    mkMeasure("Angle", "angle"),
    mkMeasure("Dihedral", "dihedral"),
    measureClear,
    measureLabel,
  );
  molView.appendChild(measureBar);
  const vibBar = el("div", "slice-bar hidden");
  vibBar.id = "vib-bar";
  const vibLabel = el("span");
  vibLabel.textContent = "Vib";
  const vibMode = document.createElement("select");
  vibMode.id = "vib-mode";
  vibMode.addEventListener("change", () => void onVibModeChange());
  const vibPlay = el("button", "field-mode-btn");
  vibPlay.id = "vib-play";
  vibPlay.textContent = "▶ Animate";
  vibPlay.addEventListener("click", () => toggleVibrationPlay());
  vibBar.append(vibLabel, vibMode, vibPlay);
  molView.appendChild(vibBar);

  const fieldView = el("div", "viewer-panel hidden");
  fieldView.id = "field-view";
  const fieldWrap = el("div", "viewer-canvas-wrap");
  const fieldCanvas = document.createElement("canvas");
  fieldCanvas.id = "field-canvas";
  fieldWrap.appendChild(fieldCanvas);
  fieldView.appendChild(fieldWrap);
  const sliceBar = el("div", "slice-bar");
  const sliceLabel = el("span");
  sliceLabel.id = "slice-label";
  sliceLabel.textContent = "Z = 0";
  const sliceSlider = document.createElement("input");
  sliceSlider.type = "range";
  sliceSlider.id = "slice-slider";
  sliceSlider.min = "0";
  sliceSlider.max = "31";
  sliceSlider.value = "0";
  sliceSlider.addEventListener("input", () => {
    void onSliceChange(Number(sliceSlider.value));
  });
  const mode2d = el("button", "field-mode-btn");
  mode2d.textContent = "2D";
  mode2d.title = "Canvas heatmap";
  mode2d.addEventListener("click", () => {
    fieldViewer?.setViewMode("slice2d");
    mode2d.classList.add("active");
    mode3d.classList.remove("active");
    modeIso.classList.remove("active");
    document.getElementById("iso-row")?.classList.add("hidden");
  });
  const mode3d = el("button", "field-mode-btn active");
  mode3d.textContent = "3D";
  mode3d.title = "wgpu orbit slice";
  mode3d.addEventListener("click", () => {
    fieldViewer?.setViewMode("slice3d");
    mode3d.classList.add("active");
    mode2d.classList.remove("active");
    modeIso.classList.remove("active");
    document.getElementById("iso-row")?.classList.add("hidden");
  });
  const modeIso = el("button", "field-mode-btn");
  modeIso.textContent = "Iso";
  modeIso.title = "Marching-cubes isosurface";
  modeIso.addEventListener("click", () => {
    fieldViewer?.setViewMode("isosurface");
    modeIso.classList.add("active");
    mode2d.classList.remove("active");
    mode3d.classList.remove("active");
    isoRow.classList.remove("hidden");
  });
  const fieldFitBtn = el("button", "field-mode-btn");
  fieldFitBtn.textContent = "Fit";
  fieldFitBtn.title = "Reset camera";
  fieldFitBtn.addEventListener("click", () => fieldViewer?.resetView());
  const isoRow = el("div", "slice-bar");
  isoRow.id = "iso-row";
  isoRow.classList.add("hidden");
  const isoLabel = el("span");
  isoLabel.id = "iso-label";
  isoLabel.textContent = "Iso 35%";
  const isoSlider = document.createElement("input");
  isoSlider.type = "range";
  isoSlider.id = "iso-slider";
  isoSlider.min = "5";
  isoSlider.max = "95";
  isoSlider.value = "35";
  isoSlider.addEventListener("input", () => {
    const level = Number(isoSlider.value) / 100;
    fieldViewer?.setIsoLevel(level);
    const sign = fieldViewer?.getIsoSign() ?? 1;
    isoLabel.textContent = `Iso ${sign > 0 ? "+" : "-"}${isoSlider.value}%`;
  });
  const isoPlus = el("button", "field-mode-btn");
  isoPlus.id = "iso-plus";
  isoPlus.textContent = "+";
  isoPlus.title = "Positive MO lobe isosurface";
  isoPlus.addEventListener("click", () => {
    fieldViewer?.setIsoSign(1);
    const slider = document.getElementById("iso-slider") as HTMLInputElement;
    isoLabel.textContent = `Iso +${slider?.value ?? "35"}%`;
  });
  const isoMinus = el("button", "field-mode-btn");
  isoMinus.id = "iso-minus";
  isoMinus.textContent = "−";
  isoMinus.title = "Negative MO lobe isosurface";
  isoMinus.addEventListener("click", () => {
    fieldViewer?.setIsoSign(-1);
    const slider = document.getElementById("iso-slider") as HTMLInputElement;
    isoLabel.textContent = `Iso −${slider?.value ?? "35"}%`;
  });
  isoRow.append(isoLabel, isoPlus, isoMinus, isoSlider);
  sliceBar.append(sliceLabel, sliceSlider, mode2d, mode3d, modeIso, fieldFitBtn);
  fieldView.appendChild(sliceBar);
  fieldView.appendChild(isoRow);

  viewerPane.append(viewerTabs, molView, fieldView);
  workspace.append(sidebar, center, viewerPane);
  app.appendChild(workspace);

  const outputPanel = el("div", "output-panel");
  const outputTitle = el("h2");
  outputTitle.textContent = "Output";
  const output = el("pre");
  output.id = "output";
  const jobPanel = el("div", "job-panel hidden");
  jobPanel.id = "job-panel";
  const jobPanelTitle = el("h2");
  jobPanelTitle.textContent = "Jobs";
  const jobPanelToolbar = el("div", "slice-bar");
  const jobCancelBtn = el("button", "field-mode-btn");
  jobCancelBtn.id = "job-cancel-btn";
  jobCancelBtn.textContent = "Cancel";
  jobCancelBtn.disabled = true;
  jobCancelBtn.addEventListener("click", () => void cancelChemJob());
  jobPanelToolbar.appendChild(jobCancelBtn);
  const jobPanelBody = el("pre", "job-panel-body");
  jobPanelBody.id = "job-panel-body";
  jobPanelBody.textContent = "No jobs";
  jobPanel.append(jobPanelTitle, jobPanelToolbar, jobPanelBody);
  outputPanel.append(outputTitle, output, jobPanel);
  app.appendChild(outputPanel);

  moleculeViewer = new MoleculeViewer(molCanvas);
  moleculeViewer.setMeasureCallback(refreshMeasureLabel);
  fieldViewer = new FieldViewer(fieldCanvas);
}

function initEditor() {
  editor = monaco.editor.create(document.getElementById("editor")!, {
    value: [
      "// PhysicsLang IDE",
      "// Molecules: examples/molecules/water.gjf | water.pdb | water.xyz",
      "// Fields: Demo Field toolbar or .field.json in explorer",
      "",
      "fn main() -> Int { return 0 }",
      "",
    ].join("\n"),
    language: PHYSLANG_LANGUAGE_ID,
    theme: "physlang-dark",
    automaticLayout: true,
    fontSize: 14,
    fontFamily: "Consolas, 'Courier New', monospace",
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
  });

  editor.onDidChangeModelContent(() => {
    dirty = true;
    updateFileLabel();
    scheduleDiagnostics?.();
  });

  scheduleDiagnostics = registerLspProviders(
    () => editor.getValue(),
    (diags) => {
      const model = editor.getModel();
      if (model) applyDiagnostics(model, diags);
    },
    () => projectRoot,
    (path) => loadFile(path),
  );
}

buildShell();
initEditor();
refreshProjectTree();
setOutput("Ready — GaussView-style: .gjf/.com, .pdb, .xyz | Demo Field for volumes.");
