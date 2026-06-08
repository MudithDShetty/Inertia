# Agent handoff — PhysicsLang / Inertia

**Repo:** `C:\Users\mudit\OneDrive\Desktop\Inertia`  
**Read first:** [Todo.md](../Todo.md), [.cursor/rules/physicslang-project.mdc](../.cursor/rules/physicslang-project.mdc), [docs/language-reference.md](language-reference.md)  
**Do not edit:** `.cursor/plans/physicslang_hub_plan_0e35861e.plan.md`  
**Only commit when user asks.** Update Todo.md when items are truly done.

---

## What this project is

Monorepo: **standalone `.phys` compiled language** (Rust + Tree-sitter + LLVM) + Python FFI, **Tauri desktop IDE**, **PhysicsViz** (wgpu offscreen → PNG → webview). First MVP vertical: quantum computing (Qiskit/PennyLane). Active sprint: **Phase 3** — IDE + GaussView-style chemistry GUI + docs.

**Do not reverse without user approval:** standalone `.phys` + Python FFI; PhysicsLab as separate extension; PhysicsOS deferred.

---

## Architecture (viz — critical)

```
Orbit/zoom in apps/ide/src/{molecule-viewer,field-viewer}.ts
  → Tauri render_molecule_frame / render_field_frame / pick_molecule_atom_cmd
  → physlang-viz/wgpu_render/ + pick.rs
  → shared_device() (one wgpu device per process)
  → PNG → <img class="wgpu-frame">
```

| Feature | Behavior |
|---------|----------|
| Molecule | wgpu on first load; debounced refresh on orbit; measure mode uses ray-pick + transparent overlay |
| Field MO | `\|mo:N` paths → dual red/blue lobes (`render_field_mo_isosurface_png`) |
| Field ESP | Quantum Hartree from ρ when fchk has basis+SCF; else classical monopole |
| Field slice3d | wgpu primary; canvas tilted fallback if wgpu fails |
| Vibration | `loadDirect()` — canvas only |

Virtual field paths (parsed in `load_scalar_field()` in `lib.rs`):  
`{fchk}|density`, `{fchk}|mo:N`, `{fchk}|esp`

---

## Completed recently (resume after this)

### Session A — Phase 3 chemistry + LSP core
- [x] Quantum ESP (Hartree from electron density ρ) in `fchk_grid.rs`
- [x] Dual-lobe MO coloring in wgpu (`render_field_mo_isosurface_png`)
- [x] LSP stdlib go-to-definition (`stdlib.rs`, walk up to `stdlib/core.phys`)
- [x] LSP code actions stub (`code_actions.rs`, Monaco provider)

### Session B — Viewer + LSP polish
- [x] Molecule wgpu load race fixed (`setMeasureMode("off")` no longer locks canvas)
- [x] Molecule debounced wgpu refresh on orbit (like field viewer)
- [x] wgpu-aligned atom picking (`physlang-viz/src/pick.rs`, `pick_molecule_atom_cmd`)
- [x] Measure overlay on wgpu (transparent canvas + gold rings)
- [x] Field canvas 3D slice fallback (`drawSlice3dCanvas`)
- [x] Stdlib in project tree + stdlib completions (`complete_phys_prefix` + `projectRoot`)
- [x] Demo pack: `examples/demo/` + this handoff doc

---

## Known issues

| Issue | Workaround |
|-------|------------|
| Port 1420 conflict | Kill old `tauri dev` process |
| `water.fchk` minimal | No basis → promolecule only; use **`water_sto3g.fchk`** for MO/ESP/GTO density |
| Gaussian/ORCA jobs | Need `GAUSSIAN_EXE` / `ORCA_EXE` env vars |
| `cargo` on PATH (Windows) | `$env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path` |
| Quantum Run in IDE | Full exec needs Python + Qiskit; `phys check` works without |
| LSP rename/refs | Same-name, file-local only |
| ESP | Numerical Hartree grid (not analytical Poisson solver) |

---

## Key paths

| Area | Path |
|------|------|
| IDE frontend | `apps/ide/src/{main.ts,molecule-viewer.ts,field-viewer.ts,lsp.ts,wgpu-viewer.ts}` |
| Tauri backend | `apps/ide/src-tauri/src/{lib.rs,chem_jobs.rs}` |
| Viz / chem | `physlang/physlang-viz/src/{fchk,fchk_basis,fchk_grid,pick.rs,cube,marching_cubes,wgpu_render/}` |
| LSP | `physlang/physlang-lsp/src/{symbols,stdlib,code_actions,completion,diagnostics}.rs` |
| Docs | `docs/{language-reference.md,quickstart.md,AGENT-HANDOFF.md}` |
| Demo / smoke | `examples/demo/{SMOKE-TEST.md,smoke-test.ps1,lsp_demo.phys}` |
| Test fixtures | `examples/molecules/water_sto3g.fchk`, `water.gjf`, `water_density.cube` |
| Master todo | [Todo.md](../Todo.md) |

---

## Verify (automated)

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path
cd C:\Users\mudit\OneDrive\Desktop\Inertia
.\examples\demo\smoke-test.ps1
```

Or manually:

```powershell
cargo test -p physlang-lsp
cargo test -p physlang-viz --features wgpu
cargo check -p physlang-ide
cd apps/ide && npm run build && npm run tauri dev
```

**Manual IDE checklist:** [examples/demo/SMOKE-TEST.md](../examples/demo/SMOKE-TEST.md)

---

## Next work (priority — from Todo.md)

1. Expand docs: stdlib auto-docs, tutorial notebooks  
2. Volume rendering stub + export PNG/MP4/VTK  
3. Structure builder / Z-matrix editor in IDE  
4. Notebook interface (`.phys` + Python cells)  
5. LSP: richer code actions (unit suffix on literals)  
6. Package hub stub, debugger DAP stub  

Phase 3 exit criteria are met; remaining items are stretch / Phase 4 prep.

---

## Agent rules

- Minimize diff scope; match existing conventions  
- Place code per Todo.md architecture table  
- Continue Phase 3 unless user says otherwise  
- User may not know `.phys` syntax — point to `docs/language-reference.md`  
- **Do not commit** unless user asks  
- Update Todo.md when items are truly done  

---

## Quick smoke (5 min)

1. `cd apps/ide && npm run tauri dev`  
2. **Open Folder** → repo root  
3. `examples/molecules/water.gjf` → orbit → **Dist** measure  
4. `water_sto3g.fchk` → Surfaces → **HOMO** → Field Iso view  
5. `examples/demo/lsp_demo.phys` → F12 on `abs`  
6. **Docs** button  

---

*Handoff generated: 2026-06-08. Copy this file + Todo.md to onboard the next agent.*
