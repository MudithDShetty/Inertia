//! Generate Markdown reference from stdlib `.phys` doc comments.

use crate::symbols::{collect_symbols, SymbolKind};
use std::fs;
use std::path::Path;

/// Collect consecutive `//` comment lines immediately above `line` (1-based).
pub fn doc_lines_before(source: &str, line: u32) -> Vec<String> {
    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() {
        return Vec::new();
    }
    let mut idx = line.saturating_sub(1) as usize;
    if idx >= lines.len() {
        idx = lines.len() - 1;
    }
    let mut docs = Vec::new();
    while idx > 0 {
        let prev = lines[idx - 1].trim();
        if let Some(text) = prev.strip_prefix("//") {
            docs.push(text.trim_start_matches('/').trim().to_string());
            idx -= 1;
        } else if prev.is_empty() {
            idx -= 1;
        } else {
            break;
        }
    }
    docs.reverse();
    docs
}

fn kind_label(kind: SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Function => "fn",
        SymbolKind::Variable => "let",
        SymbolKind::QReg => "qreg",
        SymbolKind::Extern => "extern",
        SymbolKind::Parameter => "param",
    }
}

/// Build Markdown for all `.phys` files in `stdlib_dir`.
pub fn generate_stdlib_markdown(stdlib_dir: &Path) -> String {
    let mut out = String::from("# Inertia standard library\n\n");
    out.push_str(
        "Auto-generated from `stdlib/*.phys` line comments (`//` above each symbol).\n\n",
    );
    out.push_str("See also [language reference](language-reference.md#standard-library).\n\n");

    let Ok(entries) = fs::read_dir(stdlib_dir) else {
        out.push_str("*Could not read stdlib directory.*\n");
        return out;
    };

    let mut files: Vec<_> = entries.flatten().collect();
    files.sort_by_key(|e| e.path());

    for entry in files {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("phys") {
            continue;
        }
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        let fname = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file.phys");
        out.push_str(&format!("## `{fname}`\n\n"));

        let source_lines: Vec<&str> = source.lines().collect();
        let mut syms: Vec<_> = collect_symbols(&source)
            .into_iter()
            .filter(|s| s.is_definition && !matches!(s.kind, SymbolKind::Parameter))
            .collect();
        syms.sort_by_key(|s| s.line);

        if syms.is_empty() {
            out.push_str("*No top-level symbols.*\n\n");
            continue;
        }

        for sym in syms {
            let docs = doc_lines_before(&source, sym.line);
            let sig_line = source_lines
                .get(sym.line.saturating_sub(1) as usize)
                .map(|l| l.trim())
                .unwrap_or(&sym.name);
            out.push_str(&format!(
                "### `{}` ({kind})\n\n",
                sym.name,
                kind = kind_label(sym.kind)
            ));
            if !docs.is_empty() {
                for d in &docs {
                    out.push_str(&format!("{d}\n\n"));
                }
            }
            out.push_str("```phys\n");
            out.push_str(sig_line);
            out.push_str("\n```\n\n");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn generates_markdown_for_repo_stdlib() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../stdlib");
        if !root.join("core.phys").is_file() {
            return;
        }
        let md = generate_stdlib_markdown(&root);
        assert!(md.contains("# Inertia standard library"));
        assert!(md.contains("core.phys"));
        assert!(md.contains("abs"));
    }

    #[test]
    fn doc_lines_before_fn() {
        let src = "// Absolute value\n// Returns |x|\nfn abs(x: Float) -> Float { return x }";
        let docs = doc_lines_before(src, 3);
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0], "Absolute value");
    }
}
