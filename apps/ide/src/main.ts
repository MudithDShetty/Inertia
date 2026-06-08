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
import { MoleculeViewer, type MoleculeData } from "./molecule-viewer";
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

async function loadMolecule(path: string) {
  try {
    const mol = await invoke<MoleculeData>("parse_molecule_file", { path });
    currentMoleculePath = path;
    currentFieldPath = null;
    moleculeViewer?.load(mol);
    switchViewerTab("molecule");
    setViewerVisible(true);
    highlightTreeSelection(path);
    setOutput(`Loaded molecule: ${mol.name} (${mol.atoms.length} atoms)`, "success");
  } catch (err) {
    setOutput(String(err), "error");
  }
}

async function applyFieldSlice(slice: FieldSliceData) {
  currentFieldPath = slice.path;
  currentFieldDepth = slice.depth;
  currentMoleculePath = null;
  fieldViewer?.load(slice, slice.depth - 1);
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

async function loadField(path: string) {
  try {
    const slice = await invoke<FieldSliceData>("load_field_file", { path });
    await applyFieldSlice(slice);
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
    fieldViewer?.load(slice, slice.depth - 1);
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
  updateFileLabel();
  highlightTreeSelection(path);
  scheduleDiagnostics?.();
}

async function openProjectEntry(file: ProjectFile) {
  if (file.kind === "xyz" || file.kind === "pdb") {
    await loadMolecule(file.path);
  } else if (file.kind === "field") {
    await loadField(file.path);
  } else {
    await loadFile(file.path);
  }
}

function fileTreeIcon(file: ProjectFile): string {
  switch (file.kind) {
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
    hint.textContent = "Open a folder to browse .phys, .xyz, .pdb, .field.json";
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
      { name: "Molecules", extensions: ["xyz", "pdb"] },
    ],
  });
  if (typeof selected === "string") await loadMolecule(selected);
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
    mkBtn("Save", () => void saveCurrentFile(), true),
    mkBtn("Check", () => void checkCurrent(), true),
    mkBtn("Run", () => void runCurrent()),
  );
  const spacer = el("div", "spacer");
  const fileLabel = el("span", "file-label");
  fileLabel.id = "file-label";
  toolbar.append(spacer, fileLabel);
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
  });
  const mode3d = el("button", "field-mode-btn active");
  mode3d.textContent = "3D";
  mode3d.title = "wgpu orbit slice";
  mode3d.addEventListener("click", () => {
    fieldViewer?.setViewMode("slice3d");
    mode3d.classList.add("active");
    mode2d.classList.remove("active");
    modeIso.classList.remove("active");
  });
  const modeIso = el("button", "field-mode-btn");
  modeIso.textContent = "Iso";
  modeIso.title = "Marching-cubes isosurface stub";
  modeIso.addEventListener("click", () => {
    fieldViewer?.setViewMode("isosurface");
    modeIso.classList.add("active");
    mode2d.classList.remove("active");
    mode3d.classList.remove("active");
  });
  sliceBar.append(sliceLabel, sliceSlider, mode2d, mode3d, modeIso);
  fieldView.appendChild(sliceBar);

  viewerPane.append(viewerTabs, molView, fieldView);
  workspace.append(sidebar, center, viewerPane);
  app.appendChild(workspace);

  const outputPanel = el("div", "output-panel");
  const outputTitle = el("h2");
  outputTitle.textContent = "Output";
  const output = el("pre");
  output.id = "output";
  outputPanel.append(outputTitle, output);
  app.appendChild(outputPanel);

  moleculeViewer = new MoleculeViewer(molCanvas);
  fieldViewer = new FieldViewer(fieldCanvas);
}

function initEditor() {
  editor = monaco.editor.create(document.getElementById("editor")!, {
    value: [
      "// PhysicsLang IDE",
      "// Molecules: examples/molecules/water.xyz | water.pdb",
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
  );
}

buildShell();
initEditor();
refreshProjectTree();
setOutput("Ready — water.pdb/xyz for molecules, Demo Field for scalar heatmap.");
