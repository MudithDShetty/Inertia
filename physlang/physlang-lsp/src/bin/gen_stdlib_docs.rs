//! Write `docs/stdlib-reference.md` from stdlib doc comments.

use physlang_lsp::{find_stdlib_dir, generate_stdlib_markdown};
use std::env;
use std::path::PathBuf;

fn main() {
    let root = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
        });
    let stdlib = find_stdlib_dir(&root).unwrap_or_else(|| {
        eprintln!("stdlib/ not found under {}", root.display());
        std::process::exit(1);
    });
    let md = generate_stdlib_markdown(&stdlib);
    let out = root.join("docs/stdlib-reference.md");
    std::fs::write(&out, md).unwrap_or_else(|e| {
        eprintln!("write {}: {e}", out.display());
        std::process::exit(1);
    });
    println!("Wrote {}", out.display());
}
