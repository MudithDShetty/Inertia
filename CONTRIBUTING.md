# Contributing to Inertia

## Dev setup

1. Install Rust via [rustup](https://rustup.rs)
2. Clone the repo and build: `cargo build`
3. Run tests: `cargo test`
4. Python bindings: `maturin develop`

## Crate boundaries

- **physlang-parser** — AST only; no type information
- **physlang-types** — SI units + type checker; depends on parser
- **physlang-mir** — lowering + autodiff transforms
- **physlang-runtime** — interpreter; depends on mir + quantum
- **physlang-quantum** — circuit IR, Hamiltonians, expectation values
- **physlang-llvm** — optional LLVM codegen (`--features llvm`)
- **physlang-cli** — binary `phys`; orchestrates all crates

## Adding a feature

1. Update [Todo.md](Todo.md) checkbox when done
2. Add tests in the relevant crate
3. Add an example under `examples/` if user-facing

## Code style

- Rust 2021 edition, `clippy` clean
- Minimal scope — match existing patterns in each crate
