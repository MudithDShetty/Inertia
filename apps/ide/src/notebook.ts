/** Mixed .phys + Python notebook cells (Phase 3). */

import * as monaco from "monaco-editor";
import { invoke } from "@tauri-apps/api/core";
import { PHYSLANG_LANGUAGE_ID } from "./physlang-monarch";

export interface NotebookRunResult {
  phys: { stdout: string[]; result?: string; error?: string; backend: string };
  python: { stdout: string[]; result?: string; error?: string; backend: string };
}

export interface NotebookDocument {
  version: 1;
  title: string;
  phys: string;
  python: string;
}

export class NotebookPanel {
  private root: HTMLElement;
  private physHost: HTMLElement;
  private pyHost: HTMLElement;
  private physEditor: monaco.editor.IStandaloneCodeEditor;
  private pyEditor: monaco.editor.IStandaloneCodeEditor;
  private visible = false;
  private title = "Untitled notebook";

  constructor(container: HTMLElement) {
    this.root = document.createElement("div");
    this.root.id = "notebook-panel";
    this.root.className = "notebook-panel hidden";

    const physLabel = document.createElement("div");
    physLabel.className = "notebook-cell-label";
    physLabel.textContent = "Inertia (.phys)";
    this.physHost = document.createElement("div");
    this.physHost.className = "notebook-cell-editor";

    const pyLabel = document.createElement("div");
    pyLabel.className = "notebook-cell-label";
    pyLabel.textContent = "Python";
    this.pyHost = document.createElement("div");
    this.pyHost.className = "notebook-cell-editor";

    this.root.append(physLabel, this.physHost, pyLabel, this.pyHost);
    container.appendChild(this.root);

    this.physEditor = monaco.editor.create(this.physHost, {
      value: [
        "// Notebook cell — Inertia",
        "fn main() -> Int {",
        "    return 0",
        "}",
      ].join("\n"),
      language: PHYSLANG_LANGUAGE_ID,
      theme: "physlang-dark",
      automaticLayout: true,
      fontSize: 13,
      minimap: { enabled: false },
    });

    this.pyEditor = monaco.editor.create(this.pyHost, {
      value: [
        "# Python cell — Qiskit / NumPy / matplotlib",
        "import json",
        'print(json.dumps({"ok": True, "note": "stub cell"}))',
      ].join("\n"),
      language: "python",
      theme: "vs-dark",
      automaticLayout: true,
      fontSize: 13,
      minimap: { enabled: false },
    });
  }

  isVisible() {
    return this.visible;
  }

  setVisible(on: boolean) {
    this.visible = on;
    this.root.classList.toggle("hidden", !on);
    if (on) {
      this.physEditor.layout();
      this.pyEditor.layout();
    }
  }

  toDocument(): NotebookDocument {
    return {
      version: 1,
      title: this.title,
      phys: this.physEditor.getValue(),
      python: this.pyEditor.getValue(),
    };
  }

  loadDocument(doc: NotebookDocument) {
    this.title = doc.title || "Notebook";
    this.physEditor.setValue(doc.phys);
    this.pyEditor.setValue(doc.python);
  }

  getTitle() {
    return this.title;
  }

  setTitle(t: string) {
    this.title = t;
  }

  async runAll(): Promise<NotebookRunResult> {
    const physSource = this.physEditor.getValue();
    const pySource = this.pyEditor.getValue();
    const phys = await invoke<NotebookRunResult["phys"]>("run_phys_source", {
      source: physSource,
      entry: null,
    });
    const python = await invoke<NotebookRunResult["python"]>("run_python_snippet", {
      source: pySource,
    });
    return { phys, python };
  }
}
