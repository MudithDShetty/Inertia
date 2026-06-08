import * as vscode from "vscode";
import { execFile } from "child_process";
import { promisify } from "util";

const execFileAsync = promisify(execFile);

export function activate(context: vscode.ExtensionContext) {
  const diagCollection = vscode.languages.createDiagnosticCollection("physlang");

  async function checkDocument(doc: vscode.TextDocument) {
    if (doc.languageId !== "physlang") return;
    try {
      const { stdout } = await execFileAsync("phys", ["lsp", doc.uri.fsPath], {
        timeout: 10000,
      });
      const diags: vscode.Diagnostic[] = [];
      for (const line of stdout.split("\n").filter(Boolean)) {
        const m = line.match(/:(\d+):(\d+): (.+)$/);
        if (m) {
          const ln = parseInt(m[1], 10) - 1;
          const col = parseInt(m[2], 10) - 1;
          const range = new vscode.Range(ln, col, ln, col + 1);
          diags.push(new vscode.Diagnostic(range, m[3], vscode.DiagnosticSeverity.Error));
        }
      }
      diagCollection.set(doc.uri, diags);
    } catch {
      // phys CLI not installed — silent
    }
  }

  context.subscriptions.push(
    diagCollection,
    vscode.workspace.onDidSaveTextDocument(checkDocument),
    vscode.workspace.onDidOpenTextDocument(checkDocument),
    vscode.commands.registerCommand("physlang.check", () => {
      const editor = vscode.window.activeTextEditor;
      if (editor) checkDocument(editor.document);
    })
  );
}

export function deactivate() {}
