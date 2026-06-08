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



interface IdeLocation {

  line: number;

  column: number;

  end_column: number;

  file?: string;

}



interface IdeTextEdit {

  line: number;

  column: number;

  end_column: number;

  new_text: string;

}



interface IdeCodeAction {

  title: string;

  edits: IdeTextEdit[];

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

  getProjectRoot: () => string | null,

  openFile: (path: string) => void | Promise<void>,

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

        source: model.getValue(),

        prefix: word.word,

        projectRoot: getProjectRoot(),

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



  monaco.languages.registerDefinitionProvider(PHYSLANG_LANGUAGE_ID, {

    async provideDefinition(model, position) {

      const loc = await invoke<IdeLocation | null>("goto_phys_definition", {

        source: model.getValue(),

        line: position.lineNumber,

        column: position.column,

        projectRoot: getProjectRoot(),

      });

      if (!loc) return null;

      const uri = loc.file

        ? monaco.Uri.file(loc.file)

        : model.uri;

      if (loc.file) {

        void openFile(loc.file);

      }

      return {

        uri,

        range: new monaco.Range(loc.line, loc.column, loc.line, loc.end_column),

      };

    },

  });



  monaco.languages.registerReferenceProvider(PHYSLANG_LANGUAGE_ID, {

    async provideReferences(model, position) {

      const refs = await invoke<IdeLocation[]>("find_phys_references", {

        source: model.getValue(),

        line: position.lineNumber,

        column: position.column,

      });

      return refs.map((r) => ({

        uri: model.uri,

        range: new monaco.Range(r.line, r.column, r.line, r.end_column),

      }));

    },

  });



  monaco.languages.registerRenameProvider(PHYSLANG_LANGUAGE_ID, {

    async provideRenameEdits(model, position, newName) {

      const edits = await invoke<IdeTextEdit[]>("rename_phys_symbol", {

        source: model.getValue(),

        line: position.lineNumber,

        column: position.column,

        newName,

      });

      return {

        edits: edits.map((e) => ({

          resource: model.uri,

          versionId: model.getVersionId(),

          textEdit: {

            range: new monaco.Range(e.line, e.column, e.line, e.end_column),

            text: e.new_text,

          },

        })),

      };

    },

  });



  monaco.languages.registerCodeActionProvider(PHYSLANG_LANGUAGE_ID, {

    async provideCodeActions(model, range) {

      const actions = await invoke<IdeCodeAction[]>("phys_code_actions", {

        source: model.getValue(),

        line: range.startLineNumber,

        column: range.startColumn,

      });

      if (actions.length === 0) return { actions: [], dispose: () => {} };

      return {

        actions: actions.map((a) => ({

          title: a.title,

          kind: "quickfix",

          edit: a.edits.length

            ? {

                edits: a.edits.map((e) => ({

                  resource: model.uri,

                  versionId: model.getVersionId(),

                  textEdit: {

                    range: new monaco.Range(

                      e.line,

                      e.column,

                      e.line,

                      e.end_column,

                    ),

                    text: e.new_text,

                  },

                })),

              }

            : undefined,

        })),

        dispose: () => {},

      };

    },

  });



  return scheduleCheck;

}

