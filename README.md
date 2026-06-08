# PhysicsLang

**All-in-one physics programming language** — standalone `.phys` files, SI unit type system, native autodiff, quantum computing (Qiskit/PennyLane), and a path to simulation, visualization, and lab instrumentation.

## Documentation

| Guide | Description |
|-------|-------------|
| **[docs/quickstart.md](docs/quickstart.md)** | Install, first program, CLI, IDE |
| **[docs/language-reference.md](docs/language-reference.md)** | Full `.phys` syntax — types, SI units, quantum, stdlib |
| **[docs/architecture.md](docs/architecture.md)** | Nine-layer stack |
| **[Todo.md](Todo.md)** | Roadmap and task status |

Open **Docs** in the desktop IDE to read the language reference in-editor.

## Quick start

### Prerequisites

- Rust 1.75+ ([rustup](https://rustup.rs))
- Python 3.10+ (for bindings)
- Optional: LLVM 18+ for `--features llvm` codegen

### Build & run

```bash
cargo build --release
cargo run -- run examples/hello.phys
cargo run -- check examples/quantum/h2_vqe.phys
cargo test
```

### Python bindings

```bash
pip install maturin
maturin develop --release
python -c "import physlang; print(physlang.run('examples/quantum/h2_vqe.phys'))"
```

## Example — H2 VQE (~30 lines vs ~150 Python+Qiskit)

```phys
qreg q[2]
let H = -0.39 * X(0) @ X(1) + 0.18 * Z(0) + 0.18 * Z(1)

@differentiable
fn energy(theta: Angle[4]) -> Energy {
    RY(theta, 0)
    RY(theta, 1)
    CNOT(0, 1)
    let circuit = ansatz(q, theta)
    return expect(circuit, H)
}

fn main() -> Energy {
    return energy(0.1)
}
```

```bash
phys run examples/quantum/h2_vqe.phys
# => -1.137... (Ha)
```

## CLI commands

| Command | Description |
|---------|-------------|
| `phys run FILE` | Compile + interpret |
| `phys check FILE` | Type-check (SI units) |
| `phys build FILE [--emit mir\|llvm\|ast]` | Emit artifacts |
| `phys viz FILE -o circuit.svg` | Render circuit diagram |
| `phys lsp FILE` | Print diagnostics |
| `phys repl` | Interactive read-eval-print loop stub |

## Architecture

Nine-layer stack — see [docs/architecture.md](docs/architecture.md) and [Todo.md](Todo.md).

```
IDE → Compiler (units, autodiff, LLVM) → PhysicsMath → PhysicsSim → PhysicsAtom
  → PhysicsViz → PhysicsLab → Interop → Hardware (CPU/CUDA/Metal/MPI)
```

## PhysicsLang IDE (desktop)

Minimal Tauri + Monaco shell at `apps/ide/` — open `.phys` files, project explorer, syntax highlighting (same grammar as `extensions/vscode-physlang`), **Run** via native interpreter (`physlang-runtime`, same as `phys run`), with CLI/Python fallbacks.

```powershell
cd apps/ide
npm install
npm run tauri dev
```

Open the Inertia repo root as a folder to browse `examples/` and `stdlib/`. Use **Run** on `examples/hello.phys` or **Check** for inline diagnostics. Click **Docs** for the language reference. Open `examples/molecules/water.pdb` or `water_sto3g.fchk` for 3D chemistry viewers.

## Project layout

```
physlang/           Rust compiler crates
bindings/python/    PyO3 module (pip install physlang)
stdlib/             .phys standard library
examples/           hello.phys, quantum demos
docs/               quickstart + language reference
extensions/         VS Code extension
apps/ide/           Tauri + Monaco desktop IDE (Phase 3)
```

## License

MIT OR Apache-2.0
