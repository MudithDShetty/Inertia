# Quick start

PhysicsLang programs are plain text files with the `.phys` extension. The compiler checks **SI dimensions** (you cannot add a force to a velocity) and provides a **native quantum syntax** that maps to Qiskit/PennyLane via Python FFI.

## Prerequisites

- **Rust 1.75+** — [rustup.rs](https://rustup.rs)  
- **Python 3.10+** — optional, for `pip install maturin` and quantum backends  
- **Node.js 18+** — for the desktop IDE only  

## Build the compiler

```bash
cd Inertia   # repository root
cargo build --release
```

The `phys` CLI is available via:

```bash
cargo run -- run examples/hello.phys
cargo run -- check examples/quantum/h2_vqe.phys
cargo test
```

## Your first program

Create `hello.phys`:

```phys
fn main() -> Int {
    let v: Velocity = 9.8 m/s
    let ke: Energy = 1.0 J
    return 0
}
```

Run it:

```bash
cargo run -- run hello.phys
```

Type-check only (shows unit errors without running):

```bash
cargo run -- check hello.phys
```

If you assign incompatible units, `check` reports a **dimension mismatch** with line and column — for example `let bad: Force = 1.0 J` fails because energy and force have different SI dimensions.

## CLI commands

| Command | Purpose |
|---------|---------|
| `phys run FILE` | Type-check and execute (interpreter / runtime) |
| `phys check FILE` | Type-check only |
| `phys build FILE [--emit ast\|mir\|llvm]` | Emit compiler artifacts |
| `phys viz FILE -o circuit.svg` | Export quantum circuit diagram |
| `phys lsp FILE` | Print diagnostics (same engine as the IDE) |
| `phys repl` | Interactive REPL stub |

## Desktop IDE

```powershell
cd apps/ide
npm install
npm run tauri dev
```

1. **Open Folder** → select the repository root  
2. Open `examples/hello.phys` → **Check** or **Run**  
3. Open `examples/molecules/water.gjf` → rotate the 3D structure  
4. Open `examples/molecules/water_sto3g.fchk` → **Surfaces → HOMO** for orbital isosurfaces  
5. Click **Docs** in the toolbar for the full language reference  

**Full Phase 3 checklist:** [examples/demo/SMOKE-TEST.md](../examples/demo/SMOKE-TEST.md)  
**Agent handoff:** [docs/AGENT-HANDOFF.md](AGENT-HANDOFF.md)

## Python bindings (optional)

```bash
pip install maturin
maturin develop --release
python -c "import physlang; print(physlang.run('examples/quantum/h2_vqe.phys'))"
```

## Next steps

Read the **[Language reference](language-reference.md)** for syntax details, the quantum gate list, attributes (`@differentiable`, `@python.import`), and the standard library layout under `stdlib/`.
