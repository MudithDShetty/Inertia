/**
 * Monaco Monarch tokenizer — mirrors extensions/vscode-physlang/syntaxes/physlang.tmLanguage.json
 */
import type * as monaco from "monaco-editor";

export const PHYSLANG_LANGUAGE_ID = "physlang";

export const physlangMonarch: monaco.languages.IMonarchLanguage = {
  defaultToken: "",
  ignoreCase: false,
  tokenizer: {
    root: [
      [/\/\/.*$/, "comment"],
      [
        /@(differentiable|python\.import|distributed|gpu|discretize|parallel)\b/,
        "attribute",
      ],
      [
        /\b(fn|let|return|qreg|extern|if|else|true|false)\b/,
        "keyword",
      ],
      [
        /\b(Int|Float|Bool|String|Velocity|Force|Mass|Energy|Action|Angle|Qubit|QReg|Gate|Circuit|Hamiltonian|Observable|Result|Void)\b/,
        "type",
      ],
      [
        /\b(H|X|Y|Z|S|T|CNOT|CZ|SWAP|RX|RY|RZ|U3|expect|sample|ansatz)\b/,
        "quantum",
      ],
      [/\b\d+(\.\d+)?([eE][+-]?\d+)?\b/, "number"],
      [/"/, { token: "string.quote", bracket: "@open", next: "@string" }],
      [/\bfn\s+(\w+)/, ["keyword", "function"]],
      [/[{}()\[\]]/, "@brackets"],
      [/[;,.]/, "delimiter"],
      [/[@]/, "operator"],
    ],
    string: [
      [/[^\\"]+/, "string"],
      [/\\./, "string.escape"],
      [/"/, { token: "string.quote", bracket: "@close", next: "@pop" }],
    ],
  },
};

export function registerPhyslangLanguage(monacoApi: typeof monaco): void {
  monacoApi.languages.register({ id: PHYSLANG_LANGUAGE_ID });

  monacoApi.languages.setMonarchTokensProvider(
    PHYSLANG_LANGUAGE_ID,
    physlangMonarch,
  );

  monacoApi.languages.setLanguageConfiguration(PHYSLANG_LANGUAGE_ID, {
    comments: { lineComment: "//" },
    brackets: [
      ["{", "}"],
      ["[", "]"],
      ["(", ")"],
    ],
    autoClosingPairs: [
      { open: "{", close: "}" },
      { open: "[", close: "]" },
      { open: "(", close: ")" },
      { open: '"', close: '"' },
    ],
    surroundingPairs: [
      { open: "{", close: "}" },
      { open: "[", close: "]" },
      { open: "(", close: ")" },
      { open: '"', close: '"' },
    ],
  });

  monacoApi.editor.defineTheme("physlang-dark", {
    base: "vs-dark",
    inherit: true,
    rules: [
      { token: "comment", foreground: "6A9955" },
      { token: "attribute", foreground: "C586C0", fontStyle: "italic" },
      { token: "keyword", foreground: "569CD6" },
      { token: "type", foreground: "4EC9B0" },
      { token: "quantum", foreground: "DCDCAA" },
      { token: "number", foreground: "B5CEA8" },
      { token: "string", foreground: "CE9178" },
      { token: "function", foreground: "DCDCAA" },
    ],
    colors: {
      "editor.background": "#1e1e1e",
    },
  });
}
