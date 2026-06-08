# PhysicsLang Phase 3 smoke test (Windows PowerShell)
# Run from repo root:  .\examples\demo\smoke-test.ps1

$ErrorActionPreference = "Stop"
$env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path
Set-Location $PSScriptRoot\..\..   # repo root

Write-Host "=== PhysicsLang smoke test ===" -ForegroundColor Cyan

Write-Host "`n[1/6] physlang-lsp tests..."
cargo test -p physlang-lsp --lib --quiet
if ($LASTEXITCODE -ne 0) { throw "physlang-lsp tests failed" }

Write-Host "[2/6] physlang-viz tests (wgpu + pick)..."
cargo test -p physlang-viz --lib pick --quiet
cargo test -p physlang-viz --features wgpu quantum_esp --quiet
cargo test -p physlang-viz --features wgpu field_mo --quiet
if ($LASTEXITCODE -ne 0) { throw "physlang-viz tests failed" }

Write-Host "[3/6] IDE backend check..."
cargo check -p physlang-ide --quiet
if ($LASTEXITCODE -ne 0) { throw "physlang-ide check failed" }

Write-Host "[4/6] Compiler hello.phys..."
cargo run --quiet -- run examples/hello.phys
if ($LASTEXITCODE -ne 0) { throw "hello.phys failed" }

Write-Host "[5/6] Compiler check quantum example..."
cargo run --quiet -- check examples/quantum/h2_vqe.phys
if ($LASTEXITCODE -ne 0) { throw "h2_vqe.phys check failed" }

Write-Host "[6/6] IDE frontend build (optional, ~30s)..."
Push-Location apps/ide
npm run build 2>&1 | Out-Null
$buildOk = $LASTEXITCODE -eq 0
Pop-Location
if (-not $buildOk) { Write-Host "  (warn) npm run build failed — run manually in apps/ide" -ForegroundColor Yellow }

Write-Host "`nAll core checks passed." -ForegroundColor Green
Write-Host "Manual IDE smoke: cd apps/ide && npm run tauri dev"
Write-Host "Then follow examples/demo/SMOKE-TEST.md"
