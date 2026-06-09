import "./styles.css";
import * as monaco from "monaco-editor";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { FieldViewer, type FieldSliceData } from "./field-viewer";
import { PlotViewer, type PlotData, type PlotMode } from "./plot-viewer";
import {
  applyDiagnostics,
  fetchDiagnostics,
  registerLspProviders,
} from "./lsp";
import { MoleculeViewer, type MoleculeData, type MeasureMode } from "./molecule-viewer";
import { NotebookPanel, type NotebookDocument } from "./notebook";
import { openStructureTable } from "./structure-editor";
import { openZMatrixEditor } from "./zmatrix-editor";
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

type ViewerTab = "molecule" | "field" | "plot";
type SidebarTab = "explorer" | "packages";

let editor: monaco.editor.IStandaloneCodeEditor;
let moleculeViewer: MoleculeViewer | null = null;
let fieldViewer: FieldViewer | null = null;
let plotViewer: PlotViewer | null = null;
let notebook: NotebookPanel | null = null;
let notebookMode = false;
let lastMoleculeData: MoleculeData | null = null;
let plotStreamStep = 0;
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
let fieldPlaybackHandle: ReturnType<typeof setInterval> | null = null;
let notebookPath: string | null = null;

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
  plotViewer?.resize();
}

function switchViewerTab(tab: ViewerTab) {
  document.getElementById("mol-view")?.classList.toggle("hidden", tab !== "molecule");
  document.getElementById("field-view")?.classList.toggle("hidden", tab !== "field");
  document.getElementById("plot-view")?.classList.toggle("hidden", tab !== "plot");
  document.getElementById("tab-molecule")?.classList.toggle("active", tab === "molecule");
  document.getElementById("tab-field")?.classList.toggle("active", tab === "field");
  document.getElementById("tab-plot")?.classList.toggle("active", tab === "plot");
  moleculeViewer?.resize();
  fieldViewer?.resize();
  plotViewer?.resize();
}

function switchSidebarTab(tab: SidebarTab) {
  document.getElementById("file-tree")?.classList.toggle("hidden", tab !== "explorer");
  document.getElementById("packages-panel")?.classList.toggle("hidden", tab !== "packages");
  document.getElementById("tab-explorer")?.classList.toggle("active", tab === "explorer");
  document.getElementById("tab-packages")?.classList.toggle("active", tab === "packages");
  if (tab === "packages") void refreshPackagesPanel();
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
    lastMoleculeData = mol;
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

function pathStem(p: string | null, fallback: string): string {
  if (!p) return fallback;
  const base = p.split(/[/\\]/).pop() ?? fallback;
  return base.replace(/\.[^.]+$/, "") || fallback;
}

async function exportViewerPng(kind: "molecule" | "field") {
  const viewer = kind === "molecule" ? moleculeViewer : fieldViewer;
  if (!viewer) return;
  const stem = pathStem(
    kind === "molecule" ? currentMoleculePath : currentFieldPath,
    kind === "molecule" ? "molecule" : "field",
  );
  try {
    const out = await viewer.exportPng(stem);
    if (out) setOutput(`Saved PNG: ${out}`, "success");
  } catch (err) {
    setOutput(String(err), "error");
  }
}

async function exportFieldVtk() {
  if (!currentFieldPath) {
    setOutput("Load a field first (Demo Field or Surfaces)", "error");
    return;
  }
  try {
    const out = await invoke<string>("export_field_vtk", { path: currentFieldPath });
    setOutput(`Exported VTK: ${out}`, "success");
  } catch (err) {
    setOutput(String(err), "error");
  }
}

async function loadDemoPlot() {
  try {
    const raw = await invoke<Record<string, unknown>>("demo_plot");
    plotViewer?.load(normalizePlotData(raw));
    plotViewer?.setMode("line");
    setViewerVisible(true);
    switchViewerTab("plot");
    setActivePlotMode("line");
    setOutput("Demo VQE convergence plot loaded", "success");
  } catch (err) {
    setOutput(String(err), "error");
  }
}

function normalizePlotData(raw: Record<string, unknown>): PlotData {
  return {
    title: String(raw.title ?? "Plot"),
    xLabel: String(raw.xLabel ?? raw.x_label ?? "x"),
    yLabel: String(raw.yLabel ?? raw.y_label ?? "y"),
    series: (raw.series as PlotData["series"]) ?? [],
  };
}

function setActivePlotMode(mode: PlotMode) {
  document.querySelectorAll("#plot-mode-bar .field-mode-btn").forEach((btn) => {
    btn.classList.toggle("active", btn.getAttribute("data-mode") === mode);
  });
}

async function loadDemoNotebook() {
  if (!projectRoot) {
    setOutput("Open Folder on the repo root first", "error");
    return;
  }
  const sep = projectRoot.includes("\\") ? "\\" : "/";
  const path = `${projectRoot}${sep}examples${sep}demo${sep}getting_started.inb`;
  try {
    const text = await invoke<string>("read_text_file", { path });
    const doc = JSON.parse(text) as import("./notebook").NotebookDocument;
    notebook?.loadDocument(doc);
    notebookPath = path;
    if (!notebookMode) toggleNotebook();
    setOutput(`Tutorial notebook loaded: ${path}\nTry Run NB, then switch Plot modes.`, "success");
  } catch (err) {
    setOutput(String(err), "error");
  }
}

async function formatCurrentFile() {
  const source = notebookMode && notebook?.isVisible()
    ? notebook.toDocument().phys
    : editor.getValue();
  try {
    const formatted = await invoke<string>("format_phys_source", { source });
    if (notebookMode && notebook?.isVisible()) {
      const doc = notebook.toDocument();
      doc.phys = formatted;
      notebook.loadDocument(doc);
    } else {
      editor.setValue(formatted);
    }
    setOutput("Formatted (trim trailing whitespace per line)", "success");
  } catch (err) {
    setOutput(String(err), "error");
  }
}

async function refreshPackagesPanel() {
  const panel = document.getElementById("packages-panel");
  if (!panel) return;
  panel.innerHTML = "";
  if (!projectRoot) {
    panel.textContent = "Open Folder to browse the package catalog.";
    return;
  }
  try {
    const catalog = await invoke<
      { id: string; name: string; description: string; builtin?: boolean }[]
    >("list_package_catalog", { root: projectRoot });
    const installed = new Set(
      await invoke<string[]>("list_installed_packages", { root: projectRoot }),
    );
    const title = el("p", "pkg-hint");
    title.textContent = "Package hub — install copies to .inertia/packages/";
    panel.appendChild(title);
    for (const pkg of catalog) {
      const row = el("div", "pkg-row");
      const label = el("span", "pkg-name");
      label.textContent = pkg.name;
      const desc = el("span", "pkg-desc");
      desc.textContent = pkg.description;
      const btn = el("button", "field-mode-btn");
      const isInstalled = installed.has(pkg.id) || Boolean(pkg.builtin);
      btn.textContent = isInstalled ? (pkg.builtin ? "Built-in" : "Installed") : "Install";
      btn.disabled = isInstalled;
      btn.addEventListener("click", () => {
        void (async () => {
          try {
            const msg = await invoke<string>("install_package", {
              root: projectRoot,
              packageId: pkg.id,
            });
            setOutput(msg, "success");
            await refreshPackagesPanel();
          } catch (err) {
            setOutput(String(err), "error");
          }
        })();
      });
      row.append(label, desc, btn);
      panel.appendChild(row);
    }
  } catch (err) {
    panel.textContent = String(err);
  }
}

function showTextModal(title: string, initial: string, onSave: (text: string) => void) {
  const overlay = el("div", "modal-overlay");
  const box = el("div", "modal-box");
  const h = el("h3");
  h.textContent = title;
  const ta = document.createElement("textarea");
  ta.className = "modal-textarea";
  ta.value = initial;
  const row = el("div", "modal-actions");
  const cancel = el("button", "field-mode-btn");
  cancel.textContent = "Cancel";
  const save = el("button", "field-mode-btn");
  save.textContent = "Save";
  cancel.addEventListener("click", () => overlay.remove());
  save.addEventListener("click", () => {
    onSave(ta.value);
    overlay.remove();
  });
  row.append(cancel, save);
  box.append(h, ta, row);
  overlay.appendChild(box);
  overlay.addEventListener("click", (e) => {
    if (e.target === overlay) overlay.remove();
  });
  document.body.appendChild(overlay);
  ta.focus();
}

async function openStructureEditor() {
  if (lastMoleculeData?.atoms.length) {
    openStructureTable(lastMoleculeData, (outPath) => {
      void loadMolecule(outPath, false);
    });
    return;
  }
  if (!currentMoleculePath) {
    setOutput("Open a molecule file first", "error");
    return;
  }
  const lower = currentMoleculePath.toLowerCase();
  if (!lower.endsWith(".gjf") && !lower.endsWith(".com")) {
    setOutput("Structure editor: open a molecule with atoms (.xyz, .gjf, …)", "error");
    return;
  }
  try {
    const source = await invoke<string>("read_text_file", { path: currentMoleculePath });
    showTextModal("Edit Gaussian input / geometry", source, (text) => {
      void (async () => {
        await invoke("write_text_file", { path: currentMoleculePath, content: text });
        await loadMolecule(currentMoleculePath!, false);
        setOutput("Geometry saved — molecule reloaded", "success");
      })();
    });
  } catch (err) {
    setOutput(String(err), "error");
  }
}

function openZMatrixModal() {
  if (!currentMoleculePath) {
    setOutput("Open a .gjf / .com file first", "error");
    return;
  }
  const lower = currentMoleculePath.toLowerCase();
  if (!lower.endsWith(".gjf") && !lower.endsWith(".com")) {
    setOutput("Z-matrix editor: Gaussian .gjf / .com only", "error");
    return;
  }
  void openZMatrixEditor(
    currentMoleculePath,
    () => {
      void loadMolecule(currentMoleculePath!, false);
      setOutput("Coordinate block saved — molecule reloaded", "success");
    },
    showTextModal,
  ).catch((err) => setOutput(String(err), "error"));
}

function openDebugEval() {
  showTextModal("Debug eval (expression → Int)", "1 + 2", (expr) => {
    void (async () => {
      try {
        const result = await invoke<RunResult>("debug_eval_phys", { expr });
        const lines = [
          "Debug eval (DAP stub)",
          `backend: ${result.backend}`,
          ...result.stdout,
          result.result ? `=> ${result.result}` : "",
          result.error ?? "",
        ].filter(Boolean);
        setOutput(lines.join("\n"), result.error ? "error" : "success");
      } catch (err) {
        setOutput(String(err), "error");
      }
    })();
  });
}

function toggleNotebook() {
  notebookMode = !notebookMode;
  document.getElementById("editor-pane")?.classList.toggle("hidden", notebookMode);
  notebook?.setVisible(notebookMode);
  if (!notebookMode) editor?.layout();
}

async function runNotebook() {
  if (!notebook) return;
  setOutput("Running notebook cells…\n");
  try {
    const { phys, python } = await notebook.runAll();
    const lines = [
      "=== Inertia (.phys) ===",
      `backend: ${phys.backend}`,
      ...phys.stdout,
      phys.result ? `=> ${phys.result}` : "",
      phys.error ?? "",
      "",
      "=== Python ===",
      `backend: ${python.backend}`,
      ...python.stdout,
      python.result ? `=> ${python.result}` : "",
      python.error ?? "",
    ].filter(Boolean);
    setOutput(lines.join("\n"), phys.error || python.error ? "error" : "success");
    streamRunToPlot(phys);
  } catch (err) {
    setOutput(String(err), "error");
  }
}

async function saveNotebook() {
  if (!notebook) return;
  let path = notebookPath;
  if (!path) {
    const selected = await save({
      filters: [{ name: "Inertia Notebook", extensions: ["inb"] }],
      defaultPath: "notebook.inb",
    });
    if (!selected) return;
    path = selected;
    notebookPath = path;
  }
  const doc = notebook.toDocument();
  doc.title = basename(path);
  await invoke("write_text_file", {
    path,
    content: JSON.stringify(doc, null, 2),
  });
  notebook.setTitle(doc.title);
  setOutput(`Notebook saved: ${path}`, "success");
}

async function openNotebookDialog() {
  const selected = await open({
    filters: [{ name: "Inertia Notebook", extensions: ["inb"] }],
    multiple: false,
  });
  if (!selected || Array.isArray(selected)) return;
  try {
    const text = await invoke<string>("read_text_file", { path: selected });
    const doc = JSON.parse(text) as NotebookDocument;
    notebook?.loadDocument(doc);
    notebookPath = selected;
    if (!notebookMode) toggleNotebook();
    setOutput(`Loaded notebook: ${selected}`, "success");
  } catch (err) {
    setOutput(String(err), "error");
  }
}

function stopFieldPlayback() {
  if (fieldPlaybackHandle) {
    clearInterval(fieldPlaybackHandle);
    fieldPlaybackHandle = null;
  }
  const btn = document.getElementById("field-play-btn");
  btn?.classList.remove("active");
  btn && ((btn as HTMLButtonElement).textContent = "Play");
}

function toggleFieldPlayback() {
  const slider = document.getElementById("slice-slider") as HTMLInputElement | null;
  const btn = document.getElementById("field-play-btn") as HTMLButtonElement | null;
  if (!slider || !currentFieldPath || currentFieldDepth < 2) {
    setOutput("Load a multi-slice field first", "error");
    return;
  }
  if (fieldPlaybackHandle) {
    stopFieldPlayback();
    return;
  }
  btn?.classList.add("active");
  if (btn) btn.textContent = "Stop";
  let idx = Number(slider.value);
  fieldPlaybackHandle = setInterval(() => {
    idx = (idx + 1) % currentFieldDepth;
    slider.value = String(idx);
    void onSliceChange(idx);
  }, 120);
}

async function exportFieldMp4() {
  if (!currentFieldPath || !fieldViewer) {
    setOutput("Open a field first", "error");
    return;
  }
  const mode = fieldViewer.getViewMode();
  if (mode === "slice2d" || mode === "volume") {
    setOutput("Field MP4: use 3D or Iso view mode", "error");
    return;
  }
  setOutput("Rendering field spin frames…");
  try {
    const out = await invoke<string>("export_field_mp4", {
      path: currentFieldPath,
      index: fieldViewer.getSliceIndex(),
      camera: fieldViewer.getCameraParams(),
      frames: 48,
      mode: mode === "isosurface" ? "isosurface" : null,
    });
    setOutput(out, "success");
  } catch (err) {
    setOutput(String(err), "error");
  }
}

async function runShellCommand(command: string) {
  const trimmed = command.trim();
  if (!trimmed) return;
  appendOutput(`\n$ ${trimmed}\n`);
  try {
    const result = await invoke<RunResult>("run_shell_command", {
      cwd: projectRoot,
      command: trimmed,
    });
    const lines = [...result.stdout, result.result ?? "", result.error ?? ""]
      .filter(Boolean)
      .join("\n");
    appendOutput(lines + "\n");
  } catch (err) {
    appendOutput(String(err) + "\n");
  }
}

function streamRunToPlot(result: RunResult) {
  const blob = [...result.stdout, result.result ?? ""].join(" ");
  const nums = [...blob.matchAll(/-?\d+\.\d+(?:[eE][+-]?\d+)?/g)].map((m) =>
    parseFloat(m[0]),
  );
  if (!nums.length) return;
  const val = nums[nums.length - 1]!;
  if (!Number.isFinite(val)) return;
  plotStreamStep += 1;
  plotViewer?.appendSample("run", plotStreamStep, val);
  setViewerVisible(true);
  switchViewerTab("plot");
}

async function exportMoleculeMp4() {
  if (!currentMoleculePath || !moleculeViewer) {
    setOutput("Open a molecule first", "error");
    return;
  }
  setOutput("Rendering 360° spin frames…");
  try {
    const out = await invoke<string>("export_molecule_mp4", {
      path: currentMoleculePath,
      camera: moleculeViewer.getCameraParams(),
      frames: 48,
      style: moleculeViewer.getRenderStyle(),
    });
    setOutput(out, "success");
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
  stopFieldPlayback();
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
      filters: [{ name: "Inertia", extensions: ["phys"] }],
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
    setOutput("Inertia language reference — see also docs/quickstart.md", "success");
    scheduleDiagnostics?.();
  } catch (err) {
    setOutput(`Could not load ${docPath}: ${err}`, "error");
  }
}

async function openStdlibDocs() {
  if (!projectRoot) {
    setOutput("Open the Inertia repo folder first (Open Folder), then click Stdlib again.", "error");
    return;
  }
  try {
    const source = await invoke<string>("stdlib_reference_markdown", { root: projectRoot });
    currentPath = null;
    currentMoleculePath = null;
    currentFieldPath = null;
    dirty = false;
    editor.setValue(source);
    const model = editor.getModel();
    if (model) {
      monaco.editor.setModelLanguage(model, "markdown");
    }
    updateFileLabel();
    setOutput("Standard library reference (generated from stdlib/*.phys comments)", "success");
    scheduleDiagnostics?.();
  } catch (err) {
    setOutput(String(err), "error");
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
      filters: [{ name: "Inertia", extensions: ["phys"] }],
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
    if (!result.error) streamRunToPlot(result);
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
    mkBtn("Notebook", () => toggleNotebook(), true),
    mkBtn("Open NB", () => void openNotebookDialog(), true),
    mkBtn("Demo NB", () => void loadDemoNotebook(), true),
    mkBtn("Save NB", () => void saveNotebook(), true),
    mkBtn("Run NB", () => void runNotebook(), true),
    mkBtn("Demo Field", () => void loadDemoField(), true),
    mkBtn("Demo Plot", () => void loadDemoPlot(), true),
    mkBtn("Docs", () => void openLanguageDocs(), true),
    mkBtn("Stdlib", () => void openStdlibDocs(), true),
    mkBtn("Debug", () => openDebugEval(), true),
    mkBtn("Run G16", () => void runGaussianJob(), true),
    mkBtn("Run ORCA", () => void runOrcaJob(), true),
    mkBtn("Save", () => void saveCurrentFile(), true),
    mkBtn("Format", () => void formatCurrentFile(), true),
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
  const sidebarTabs = el("div", "sidebar-tabs");
  const tabExplorer = el("button", "sidebar-tab active");
  tabExplorer.id = "tab-explorer";
  tabExplorer.textContent = "Explorer";
  tabExplorer.addEventListener("click", () => switchSidebarTab("explorer"));
  const tabPackages = el("button", "sidebar-tab");
  tabPackages.id = "tab-packages";
  tabPackages.textContent = "Packages";
  tabPackages.addEventListener("click", () => switchSidebarTab("packages"));
  sidebarTabs.append(tabExplorer, tabPackages);
  const fileTree = el("div", "file-tree");
  fileTree.id = "file-tree";
  const packagesPanel = el("div", "packages-panel hidden");
  packagesPanel.id = "packages-panel";
  packagesPanel.textContent = "Open Folder to list stdlib packages.";
  sidebar.append(sidebarTabs, fileTree, packagesPanel);

  const center = el("div", "center-column");
  const editorPane = el("div", "editor-pane");
  editorPane.id = "editor-pane";
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
  const tabPlot = el("button", "viewer-tab");
  tabPlot.id = "tab-plot";
  tabPlot.textContent = "Plot";
  tabPlot.addEventListener("click", () => switchViewerTab("plot"));
  viewerTabs.append(tabMol, tabField, tabPlot);

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
  const molPngBtn = el("button", "field-mode-btn");
  molPngBtn.textContent = "PNG";
  molPngBtn.title = "Save current molecule view as PNG";
  molPngBtn.addEventListener("click", () => void exportViewerPng("molecule"));
  molStyleBar.appendChild(molPngBtn);
  const editGeomBtn = el("button", "field-mode-btn");
  editGeomBtn.textContent = "Edit";
  editGeomBtn.title = "Edit coordinates (table) or .gjf text";
  editGeomBtn.addEventListener("click", () => void openStructureEditor());
  molStyleBar.appendChild(editGeomBtn);
  const zMatrixBtn = el("button", "field-mode-btn");
  zMatrixBtn.textContent = "Z-mat";
  zMatrixBtn.title = "Edit Z-matrix / Cartesian block in .gjf";
  zMatrixBtn.addEventListener("click", () => openZMatrixModal());
  molStyleBar.appendChild(zMatrixBtn);
  const molMp4Btn = el("button", "field-mode-btn");
  molMp4Btn.textContent = "MP4";
  molMp4Btn.title = "360° spin animation (PNG frames; ffmpeg for MP4)";
  molMp4Btn.addEventListener("click", () => void exportMoleculeMp4());
  molStyleBar.appendChild(molMp4Btn);
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
    modeVol.classList.remove("active");
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
    modeVol.classList.remove("active");
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
    modeVol.classList.remove("active");
    isoRow.classList.remove("hidden");
  });
  const modeVol = el("button", "field-mode-btn");
  modeVol.textContent = "Vol";
  modeVol.title = "Ray-march volume rendering (CPU stub)";
  modeVol.addEventListener("click", () => {
    fieldViewer?.setViewMode("volume");
    modeVol.classList.add("active");
    mode2d.classList.remove("active");
    mode3d.classList.remove("active");
    modeIso.classList.remove("active");
    isoRow.classList.add("hidden");
  });
  const fieldFitBtn = el("button", "field-mode-btn");
  fieldFitBtn.textContent = "Fit";
  fieldFitBtn.title = "Reset camera";
  fieldFitBtn.addEventListener("click", () => fieldViewer?.resetView());
  const fieldPngBtn = el("button", "field-mode-btn");
  fieldPngBtn.textContent = "PNG";
  fieldPngBtn.title = "Save current field view as PNG";
  fieldPngBtn.addEventListener("click", () => void exportViewerPng("field"));
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
  const fieldVtkBtn = el("button", "field-mode-btn");
  fieldVtkBtn.textContent = "VTK";
  fieldVtkBtn.title = "Export scalar field as VTK structured points";
  fieldVtkBtn.addEventListener("click", () => void exportFieldVtk());
  const fieldPlayBtn = el("button", "field-mode-btn");
  fieldPlayBtn.id = "field-play-btn";
  fieldPlayBtn.textContent = "Play";
  fieldPlayBtn.title = "Animate slice through depth (time-series stub)";
  fieldPlayBtn.addEventListener("click", () => toggleFieldPlayback());
  const fieldMp4Btn = el("button", "field-mode-btn");
  fieldMp4Btn.textContent = "MP4";
  fieldMp4Btn.title = "360° spin on 3D/Iso field view";
  fieldMp4Btn.addEventListener("click", () => void exportFieldMp4());
  sliceBar.append(sliceLabel, sliceSlider, mode2d, mode3d, modeIso, modeVol, fieldFitBtn, fieldPngBtn, fieldVtkBtn, fieldPlayBtn, fieldMp4Btn);
  fieldView.appendChild(sliceBar);
  fieldView.appendChild(isoRow);

  const plotView = el("div", "viewer-panel hidden");
  plotView.id = "plot-view";
  const plotWrap = el("div", "viewer-canvas-wrap");
  const plotCanvas = document.createElement("canvas");
  plotCanvas.id = "plot-canvas";
  plotWrap.appendChild(plotCanvas);
  plotView.appendChild(plotWrap);
  const plotModeBar = el("div", "slice-bar");
  plotModeBar.id = "plot-mode-bar";
  const mkPlotMode = (label: string, mode: PlotMode, active = false) => {
    const btn = el("button", "field-mode-btn");
    if (active) btn.classList.add("active");
    btn.textContent = label;
    btn.setAttribute("data-mode", mode);
    btn.addEventListener("click", () => {
      plotViewer?.setMode(mode);
      setActivePlotMode(mode);
    });
    return btn;
  };
  plotModeBar.append(
    mkPlotMode("Line", "line", true),
    mkPlotMode("Scatter", "scatter"),
    mkPlotMode("Hist", "histogram"),
    mkPlotMode("Contour", "contour"),
  );
  plotView.appendChild(plotModeBar);

  viewerPane.append(viewerTabs, molView, fieldView, plotView);
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
  const terminalRow = el("div", "terminal-row");
  const terminalPrompt = el("span", "terminal-prompt");
  terminalPrompt.textContent = "$";
  const terminalInput = document.createElement("input");
  terminalInput.type = "text";
  terminalInput.className = "terminal-input";
  terminalInput.placeholder = "Shell command (project root cwd)…";
  terminalInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      void runShellCommand(terminalInput.value);
      terminalInput.value = "";
    }
  });
  terminalRow.append(terminalPrompt, terminalInput);
  outputPanel.appendChild(terminalRow);
  app.appendChild(outputPanel);

  moleculeViewer = new MoleculeViewer(molCanvas);
  moleculeViewer.setMeasureCallback(refreshMeasureLabel);
  fieldViewer = new FieldViewer(fieldCanvas);
  plotViewer = new PlotViewer(plotCanvas);
  notebook = new NotebookPanel(center);
}

function initEditor() {
  editor = monaco.editor.create(document.getElementById("editor")!, {
    value: [
      "// Inertia IDE — language: Inertia (.phys)",
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
setOutput("Ready — Inertia IDE | GaussView-style chemistry | Demo Field / Plot");
