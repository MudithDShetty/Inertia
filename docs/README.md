# PhysicsLang documentation

PhysicsLang (`.phys`) is a standalone compiled language for physics — SI unit checking at compile time, native quantum circuit syntax, and Python FFI for Qiskit, PennyLane, and scientific libraries.

| Document | Audience | Description |
|----------|----------|-------------|
| **[Quick start](quickstart.md)** | New users | Install, first program, CLI, IDE in 10 minutes |
| **[Language reference](language-reference.md)** | Daily use | Complete syntax, types, units, quantum, stdlib |
| **[Architecture](architecture.md)** | Contributors | Nine-layer stack and crate boundaries |

## Recommended reading order

1. [Quick start](quickstart.md) — run `examples/hello.phys` and open the IDE  
2. [Language reference § Hello physics](language-reference.md#hello-physics) — minimal program anatomy  
3. [Language reference § SI units](language-reference.md#si-units-and-quantities) — what makes `.phys` different from Python  
4. [Language reference § Quantum computing](language-reference.md#quantum-computing) — gates, circuits, VQE/QAOA patterns  
5. Browse `examples/` and `stdlib/` alongside the reference  

## Examples index

| Path | Topic |
|------|--------|
| `examples/hello.phys` | SI unit literals and type annotations |
| `examples/quantum/h2_vqe.phys` | VQE with `@differentiable` |
| `examples/quantum/qaoa_maxcut.phys` | QAOA energy function |
| `examples/quantum/grover.phys` | Grover circuit + sampling |
| `examples/math/extern_daxpy.phys` | `extern c` FFI declaration |
| `examples/molecules/` | Chemistry files for the IDE viewer (not `.phys`) |

## Getting help in the IDE

- **Docs** toolbar button — opens this language reference in the editor  
- **Check** — inline unit and type errors  
- **Hover** — gate and SI type hints on supported symbols  
- **F12** — go to definition for user-defined symbols  
- **Shift+F12** — find references · **F2** — rename symbol  
