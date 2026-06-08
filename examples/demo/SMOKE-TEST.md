# Phase 3 IDE smoke test (manual)

Use this checklist after `npm run tauri dev` in `apps/ide/`.  
**Important:** click **Open Folder** and select the **repository root** (`Inertia/`), not only `examples/molecules` — stdlib completions, F12, and Docs need the root.

## Automated preflight

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path
cd C:\Users\mudit\OneDrive\Desktop\Inertia
.\examples\demo\smoke-test.ps1
```

## 1. Editor + LSP

| Step | File | Action | Expected |
|------|------|--------|----------|
| 1 | `examples/demo/lsp_demo.phys` | Open from tree | Syntax highlights |
| 2 | same | Wait ~1s | Red squiggle on `bad_units` return (dimension mismatch) |
| 3 | same | Ctrl+Space on `bel` | Completion `bell` from stdlib |
| 4 | same | F12 on `abs` | Opens `stdlib/core.phys` |
| 5 | same | F12 on `demo_energy` | Jump to fn definition in same file |
| 6 | same | Lightbulb on `bad_units` line | Quick-fix hint |
| 7 | `examples/hello.phys` | **Run** | Success in output panel |

## 2. Molecule viewer (wgpu + measure)

| Step | File | Action | Expected |
|------|------|--------|----------|
| 1 | `examples/molecules/water.gjf` | Open | wgpu ball-and-stick (may flash canvas first) |
| 2 | same | Drag orbit | wgpu refreshes after release |
| 3 | same | Style → space-fill | Re-render |
| 4 | same | **Dist** → click two atoms | Distance in Å (works on wgpu view) |
| 5 | `examples/molecules/water.pdb` | Open | Same viewer |
| 6 | `examples/molecules/water.xyz` | Open | Same viewer |
| 7 | `examples/molecules/water_freq.log` | Open | Geometry + vib bar if modes present |
| 8 | same | Animate mode | Canvas vibration (no wgpu) |

## 3. Surfaces / field viewer (GaussView-style)

Use **`water_sto3g.fchk`** (has GTO basis + MO coeffs).  
Do **not** use `water.fchk` alone for MO/ESP — it lacks basis (promolecule density only).

| Step | File | Action | Expected |
|------|------|--------|----------|
| 1 | `examples/molecules/water_sto3g.fchk` | Open molecule tab | Structure |
| 2 | Surfaces → **Density** | Load field tab | Scalar field |
| 3 | View → **Iso** | | Red/blue or cyan isosurface (density: single lobe) |
| 4 | Surfaces → **HOMO** | | Dual red/blue MO lobes |
| 5 | Iso **+ / −** | | Adjust level (density/ESP single lobe; MO dual by default) |
| 6 | Surfaces → **ESP** | | Label "quantum Hartree" if basis present |
| 7 | `examples/molecules/water_density.cube` | Open | Cube isosurface |
| 8 | Field → **Demo Field** (toolbar) | | Gaussian demo volume |

## 4. Docs + project tree

| Step | Action | Expected |
|------|--------|----------|
| 1 | File tree | Entries under `stdlib/` (e.g. `stdlib/core.phys`) |
| 2 | **Docs** button | Opens `docs/language-reference.md` |
| 3 | Open `stdlib/quantum.phys` | Read-only browse in editor |

## 5. Jobs (optional — needs Gaussian/ORCA on PATH)

| Step | File | Action | Expected |
|------|------|--------|----------|
| 1 | `examples/molecules/water.gjf` | Run G16 / ORCA | Queued in Jobs panel (or env error if exe missing) |

Set env before running jobs:

```powershell
$env:GAUSSIAN_EXE = "C:\path\to\g16.exe"   # or g09
$env:ORCA_EXE = "C:\path\to\orca.exe"
```

## Demo asset map

| Path | Purpose |
|------|---------|
| `examples/demo/lsp_demo.phys` | LSP: completion, F12, diagnostics, code actions |
| `examples/hello.phys` | SI units + Run |
| `examples/quantum/h2_vqe.phys` | Quantum + Run (needs Python/Qiskit for full exec) |
| `examples/molecules/water.gjf` | Gaussian input, 3D viewer |
| `examples/molecules/water_sto3g.fchk` | **HOMO/LUMO/ESP/density** (full GTO test fixture) |
| `examples/molecules/water.fchk` | Geometry/freq only (minimal fchk) |
| `examples/molecules/water_density.cube` | Cube isosurface |
| `examples/molecules/water.pdb` / `.xyz` | Structure formats |
| `examples/molecules/water_sp.log` | SCF energy + geometry |
| `examples/molecules/water_freq.log` | Frequencies + animation |
| `examples/fields/demo_gaussian.field.json` | JSON scalar field |

## Troubleshooting

- **Port 1420 in use:** kill old `tauri dev` / node process, retry.
- **cargo not found:** `$env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path`
- **wgpu blank on molecule:** orbit once or reload file; check `measure` mode is **off**.
- **MO/ESP looks wrong:** confirm `water_sto3g.fchk`, not `water.fchk`.
