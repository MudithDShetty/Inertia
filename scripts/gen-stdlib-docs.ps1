# Regenerate docs/stdlib-reference.md from stdlib/*.phys comments
$ErrorActionPreference = "Stop"
$env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path
Set-Location $PSScriptRoot\..
cargo run -q -p physlang-lsp --bin gen-stdlib-docs -- .
