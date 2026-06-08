---
name: PhysicsLang Hub Plan
overview: Build PhysicsLang as a standalone `.phys` language with Python FFI, phased from a quantum-computing MVP (Qiskit/PennyLane) toward the full 9-layer physics hub in your architecture diagram.
masterTodo: Todo.md
cursorRule: .cursor/rules/physicslang-project.mdc
todos:
  - id: scaffold-monorepo
    content: Initialize git, Rust workspace (physlang-* crates), pyproject.toml, README, CI
    status: pending
  - id: parser-grammar
    content: "Tree-sitter .phys grammar: literals, functions, attributes, quantum types"
    status: pending
  - id: unit-typechecker
    content: SI unit type system in physlang-types with compile-time dimensional errors
    status: pending
  - id: llvm-codegen
    content: "Basic LLVM codegen: functions, arithmetic, native execution via phys CLI"
    status: pending
  - id: quantum-stdlib
    content: "stdlib/quantum.phys: gates, circuits, Hamiltonians, expect/sample"
    status: pending
  - id: python-ffi
    content: PyO3 bindings + @python.import FFI for Qiskit/PennyLane/NumPy
    status: pending
  - id: quantum-autodiff
    content: MIR autodiff pass for @differentiable quantum expectation values
    status: pending
  - id: quantum-demos
    content: H2 VQE, QAOA, Grover examples with README benchmark vs raw Qiskit
    status: pending
  - id: vscode-lsp
    content: "VS Code extension + LSP: syntax highlight, diagnostics, hover"
    status: pending
  - id: viz-mvp
    content: Circuit diagram SVG renderer + matplotlib convergence plots
    status: pending
isProject: true
---

# PhysicsLang — All-in-One Physics Hub (Standalone Language)

> **Master task list:** [Todo.md](../Todo.md) — comprehensive checklist for every layer (compiler, quantum, molecular/PhysicsAtom, fluid/CFD, PhysicsSim, PhysicsViz, PhysicsLab instrumentation, interop, hardware). Cursor loads this via [.cursor/rules/physicslang-project.mdc](../.cursor/rules/physicslang-project.mdc). Update Todo.md checkboxes as work completes.

## What you are building

A **standalone compiled language** (`.phys` files) purpose-built for physics, with a **Python FFI bridge** so existing ecosystems (NumPy, Qiskit, PennyLane, SciPy) remain first-class. Long-term goal: the 9-layer stack in your diagram — IDE, compiler, math, simulation, atoms/quantum, viz, lab instrumentation, legacy interop, multi-hardware backends.

**Your choices that shape this plan:**

- **Language model:** Standalone `.phys` + Python FFI (not Taichi-style embedded DSL)
- **First killer vertical:** Quantum computing (circuits, Hamiltonians, variational algorithms, Qiskit/PennyLane execution)

See full plan content in user Cursor plans folder. This workspace copy links to [Todo.md](../Todo.md) for all implementation tasks.

## Todo.md phase index

| Phase | Section in Todo.md | Summary |
|-------|-------------------|---------|
| 0 | Foundation | Parser, SI units, LLVM, CLI, VS Code syntax |
| 1 | Quantum MVP | Qiskit/PennyLane, autodiff, VQE/QAOA/Grover, LSP |
| 2 | PhysicsMath | LA, FFT, CAS/SymPy, Fortran FFI, sim stubs |
| 3 | IDE + Viz + Atom | Tauri IDE, 3D viz, molecules, periodic table, DFT stubs |
| 4 | PhysicsSim | PDE syntax, CFD, FEM, MD, Monte Carlo, EM/FDTD, MPI |
| 5 | PhysicsLab | LabVIEW-style DAQ, dashboards, GPIB/USB, Arduino/FPGA |
| 6 | Interop | Fortran/C/MATLAB, OpenFOAM/COMSOL, package registry |
| 7 | Hardware | CUDA, Metal, WASM, profiling |
| 8 | PhysicsOS | Deferred research only |
