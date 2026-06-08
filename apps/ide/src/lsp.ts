import * as monaco from "monaco-editor";
import { invoke } from "@tauri-apps/api/core";
import { PHYSLANG_LANGUAGE_ID } from "./physlang-monarch";

export interface Diagnostic {
  line: number;
  column: number;
  message: string;
  severity: string;
}

interface Completion {
  label: string;
  detail?: string;
  insert_text: string;
  kind: string;
}

interface Hover {
  contents: string;
}

function markerSeverity(sev: string): monaco.MarkerSeverity {
  switch (sev) {
    case "warning":
      return monaco.MarkerSeverity.Warning;
    case "info":
      return monaco.MarkerSeverity.Info;
    default:
      return monaco.MarkerSeverity.Error;
  }
}

function completionKind(
  kind: string,
): monaco.languages.CompletionItemKind {
  switch (kind) {
    case "keyword":
      return monaco.languages.CompletionItemKind.Keyword;
    case "type":
      return monaco.languages.CompletionItemKind.Class;
    case "property":
      return monaco.languages.CompletionItemKind.Property;
    default:
      return monaco.languages.CompletionItemKind.Function;
  }
}

export function applyDiagnostics(
  model: monaco.editor.ITextModel,
  diags: Diagnostic[],
): void {
  const markers: monaco.editor.IMarkerData[] = diags.map((d) => ({
    severity: markerSeverity(d.severity),
    message: d.message,
    startLineNumber: d.line,
    startColumn: d.column,
    endLineNumber: d.line,
    endColumn: d.column + 1,
  }));
  monaco.editor.setModelMarkers(model, PHYSLANG_LANGUAGE_ID, markers);
}

export async function fetchDiagnostics(source: string): Promise<Diagnostic[]> {
  return invoke<Diagnostic[]>("check_phys_source", { source });
}

export function registerLspProviders(
  getSource: () => string,
  onDiagnostics: (diags: Diagnostic[]) => void,
): () => void {
  let debounce: ReturnType<typeof setTimeout> | undefined;

  const scheduleCheck = () => {
    clearTimeout(debounce);
    debounce = setTimeout(async () => {
      const diags = await fetchDiagnostics(getSource());
      onDiagnostics(diags);
    }, 400);
  };

  monaco.languages.registerDocumentFormattingEditProvider(PHYSLANG_LANGUAGE_ID, {
    provideDocumentFormattingEdits(model) {
      const formatted = model
        .getValue()
        .split("\n")
        .map((line) => line.trimEnd())
        .join("\n");
      return [
        {
          range: model.getFullModelRange(),
          text: formatted,
        },
      ];
    },
  });

  monaco.languages.registerCompletionItemProvider(PHYSLANG_LANGUAGE_ID, {
    triggerCharacters: ["@", "."],
    async provideCompletionItems(model, position) {
      const word = model.getWordUntilPosition(position);
      const items = await invoke<Completion[]>("complete_phys_prefix", {
        prefix: word.word,
      });
      const range = {
        startLineNumber: position.lineNumber,
        endLineNumber: position.lineNumber,
        startColumn: word.startColumn,
        endColumn: word.endColumn,
      };
      return {
        suggestions: items.map((item) => ({
          label: item.label,
          kind: completionKind(item.kind),
          detail: item.detail,
          insertText: item.insert_text,
          range,
        })),
      };
    },
  });

  monaco.languages.registerHoverProvider(PHYSLANG_LANGUAGE_ID, {
    async provideHover(model, position) {
      const info = await invoke<Hover | null>("hover_phys_source", {
        source: model.getValue(),
        line: position.lineNumber,
        column: position.column,
      });
      if (!info) return null;
      return {
        range: new monaco.Range(
          position.lineNumber,
          1,
          position.lineNumber,
          model.getLineMaxColumn(position.lineNumber),
        ),
        contents: [{ value: info.contents }],
      };
    },
  });

  return scheduleCheck;
}
