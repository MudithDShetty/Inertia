//! Index of symbols defined in stdlib `.phys` files.

use crate::symbols::{collect_symbols, SymbolKind};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdlibSymbol {
    pub name: String,
    pub file: PathBuf,
    pub line: u32,
    pub column: u32,
    pub end_column: u32,
    pub kind: SymbolKind,
}

/// Walk upward from `start` to locate `stdlib/core.phys`.
pub fn find_stdlib_dir(start: &Path) -> Option<PathBuf> {
    let mut cur = Some(start);
    while let Some(dir) = cur {
        let candidate = dir.join("stdlib");
        if candidate.join("core.phys").is_file() {
            return Some(candidate);
        }
        cur = dir.parent();
    }
    None
}

/// Parse all `.phys` files under `stdlib_dir` and build a name → definition map.
pub fn index_stdlib_dir(stdlib_dir: &Path) -> HashMap<String, StdlibSymbol> {
    let mut index = HashMap::new();
    let Ok(entries) = fs::read_dir(stdlib_dir) else {
        return index;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("phys") {
            continue;
        }
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        for sym in collect_symbols(&source) {
            if !sym.is_definition {
                continue;
            }
            if matches!(sym.kind, SymbolKind::Parameter) {
                continue;
            }
            index.entry(sym.name.clone()).or_insert(StdlibSymbol {
                name: sym.name,
                file: path.clone(),
                line: sym.line,
                column: sym.column,
                end_column: sym.end_column,
                kind: sym.kind,
            });
        }
    }
    index
}

pub fn stdlib_definition_at<'a>(
    index: &'a HashMap<String, StdlibSymbol>,
    name: &str,
) -> Option<&'a StdlibSymbol> {
    index.get(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexes_core_abs_from_source() {
        let source = include_str!("../../../stdlib/core.phys");
        let syms: Vec<_> = collect_symbols(source)
            .into_iter()
            .filter(|s| s.is_definition && s.name == "abs")
            .collect();
        assert_eq!(syms.len(), 1);
    }
}
