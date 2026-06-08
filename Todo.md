# PhysicsLang — Master Todo List

> **Project:** Inertia / PhysicsLang — all-in-one physics programming hub  
> **Language model:** Standalone `.phys` compiled language + Python FFI bridge  
> **First MVP vertical:** Quantum computing (Qiskit / PennyLane)  
> **Long-term vision:** 9-layer stack — IDE, compiler, math, simulation, atoms/quantum, viz, lab instrumentation, legacy interop, multi-hardware backends  
> **Plan reference:** [.cursor/plans/physicslang_hub_plan.plan.md](.cursor/plans/physicslang_hub_plan.plan.md)

Status key: `[ ]` not started · `[~]` in progress · `[x]` done

---

## Phase 0 — Foundation (weeks 1–4)

**Goal:** Compiling "hello physics" with SI unit checking.

### Monorepo & tooling
- [x] Initialize git repository
- [x] Rust Cargo workspace (`physlang-*` crates)
- [x] `pyproject.toml` + maturin for Python bindings
- [x] Root `README.md` with project vision and quickstart stub
- [x] GitHub Actions CI: `cargo test`, `cargo clippy`, example compile
- [x] `docs/architecture.md` — 9-layer diagram and data-flow docs
- [x] `CONTRIBUTING.md` — dev setup, crate boundaries, naming conventions

### Compiler core — parser & AST
- [x] Tree-sitter grammar for `.phys` file extension
- [x] Lexer tokens: identifiers, literals, operators, unicode math symbols (∇, ∂, etc.)
- [x] AST nodes: modules, functions, structs, enums, attributes, imports
- [x] `physlang-parser` crate: parse file → AST
- [x] Parser error messages with line/column spans
- [x] `phys repl` — interactive read-eval-print loop stub

### Compiler core — type system & SI units
- [x] `physlang-types` crate: type inference engine
- [x] Base SI dimensions: `[L, M, T, I, Θ, N, J]` (length, mass, time, current, temperature, amount, luminosity)
- [x] Derived unit types: `Velocity`, `Force`, `Energy`, `Pressure`, `Action`, `Angle`, etc.
- [x] Unit algebra: multiply, divide, power; compile-time simplification
- [x] Compile error: dimension mismatch (e.g. `Force = Mass + Velocity`)
- [x] Literal syntax: `9.8 m/s`, `1.054571817e-34 J*s`
- [ ] Unit conversion at compile time where safe (e.g. `km` → `m`)
- [ ] Generic `Quantity<T, Unit>` type for extensibility

### Compiler core — MIR & LLVM
- [x] `physlang-mir` crate: AST → HIR → MIR lowering
- [x] `physlang-llvm` crate: MIR → LLVM IR codegen
- [x] LLVM 18+ via Inkwell; link against system LLVM
- [x] Codegen: functions, locals, arithmetic, comparisons, returns
- [x] Native binary output via `phys build`
- [x] `phys run` — compile + execute in one step

### CLI
- [x] `physlang-cli` crate
- [x] Commands: `build`, `run`, `check`, `repl`, `fmt` (stub)
- [x] `--target cpu|cuda|metal|wasm` flag (cpu only in Phase 0)
- [x] `--emit llvm|mir|ast` debug flags

### Standard library — core (minimal)
- [x] `stdlib/core.phys` — unit constants, basic math
- [x] `examples/hello.phys` — prints + unit-checked variable

### IDE — early
- [x] VS Code extension skeleton (`extensions/vscode-physlang`)
- [x] Syntax highlighting for `.phys` (TextMate grammar or tree-sitter)
- [ ] File icon association

### Phase 0 exit criteria
- [x] `examples/hello.phys` compiles and runs
- [x] Unit mismatch produces clear compile error
- [ ] Compile time < 2s for hello example

---

## Phase 1 — Quantum MVP (weeks 5–12)

**Goal:** VQE / QAOA / Grover in `.phys`, executed via Qiskit, differentiated via PennyLane.

### Quantum language surface
- [x] Quantum types: `Qubit`, `QReg`, `Gate`, `Circuit`, `Hamiltonian`, `Observable`
- [x] Register declaration: `qreg q[n]`
- [x] Built-in gates: `H`, `X`, `Y`, `Z`, `S`, `T`, `CNOT`, `CZ`, `SWAP`
- [x] Parametric gates: `RX(θ)`, `RY(θ)`, `RZ(θ)`, `U3(θ, φ, λ)`
- [x] Circuit composition: `compose`, `@` (tensor product), sequential application
- [x] Measurement: `expect(circuit, obs)`, `sample(circuit, shots)`
- [x] `@differentiable` attribute on quantum energy functions
- [x] `@python.import("module")` FFI attribute syntax

### Quantum standard library
- [x] `stdlib/quantum.phys` — full gate set and helpers
- [x] Ansatz builders: hardware-efficient, UCCSD stub
- [x] Hamiltonian construction from Pauli strings
- [x] `physlang-quantum` Rust crate: runtime support for circuit IR

### Python FFI bridge
- [x] `bindings/python/` — PyO3 module `physlang`
- [x] `physlang.compile("file.phys")` → callable native module
- [x] Zero-copy NumPy buffer exchange for parameter vectors
- [x] `@python.import` codegen: trampoline to CPython API
- [x] Qiskit integration: export circuit IR → Qiskit `QuantumCircuit`
- [x] Qiskit execution: Aer simulator + IBM hardware backends (Aer validated; IBM hardware stub)
- [x] PennyLane integration: export differentiable nodes for VQA
- [ ] `pip install physlang` packaging via maturin + PyPI publish pipeline

### Native autodiff — quantum
- [x] MIR autodiff pass: reverse-mode through `expect()` and parametric gates
- [x] Parameter-shift rule for Pauli rotations
- [x] Adjoint method for statevector simulators
- [ ] PennyLane FFI fallback for unsupported hardware backends
- [ ] Optimizer hooks: gradient descent, Adam (via Python or native)

### PhysicsViz — quantum MVP
- [x] Circuit diagram IR (JSON) → SVG renderer
- [x] Expectation-value convergence plots (matplotlib via Python)
- [ ] 1-qubit Bloch sphere visualization
- [ ] Measurement histogram plots

### Quantum examples & benchmarks
- [x] `examples/quantum/h2_vqe.phys` — H₂ ground-state energy via VQE
- [x] `examples/quantum/qaoa_maxcut.phys` — combinatorial optimization
- [x] `examples/quantum/grover.phys` — Grover search on Aer
- [x] README benchmark: ~30 lines `.phys` vs ~150 lines Python+Qiskit
- [x] Validate H₂ energy within 1 mHa of Qiskit Nature reference

### LSP — quantum MVP
- [x] `physlang-lsp` crate (or submodule)
- [x] Diagnostics: type errors, unit errors inline
- [x] Hover documentation for gates and types
- [ ] Go-to-definition for functions and stdlib symbols

### Phase 1 exit criteria
- [x] All three quantum examples run on Qiskit Aer (H₂ VQE validated; QAOA/Grover via native interpreter)
- [x] VQE autodiff converges to reference energy
- [ ] `pip install physlang` works on Windows, macOS, Linux

---

## Phase 2 — PhysicsMath + classical stubs (months 4–6)

**Goal:** High-performance math engine; interfaces for future simulation domains.

### PhysicsMath — linear algebra
- [x] `physlang-runtime` tensor type: dense N-D arrays (`physlang-math::Tensor`)
- [x] Wrap `ndarray` + `faer` for LA operations
- [ ] BLAS/LAPACK optional backend (OpenBLAS, MKL)
- [x] Operations: matmul, solve, inverse, SVD, eigendecomposition, Cholesky
- [x] Sparse matrix type + basic ops (CSR/CSC)
- [x] Taichi-style sparse grid abstraction (design + stub)
- [x] Benchmark: 1000×1000 matmul within 2× of OpenBLAS (via `validate_phase2.py`)

### PhysicsMath — transforms & calculus
- [x] FFT / IFFT (rustfft or FFTW via FFI)
- [x] Numerical integration (trapezoid, Simpson, Gauss)
- [x] Finite-difference operators: gradient, divergence, curl, Laplacian
- [x] Tensor calculus helpers: metric tensors, Christoffel symbols (stub)

### PhysicsMath — symbolic CAS
- [x] SymPy integration via Python FFI (`@python.import("sympy")`)
- [ ] Expression type in `.phys` for symbolic manipulation
- [x] Differentiate, simplify, substitute symbolically
- [ ] Export symbolic → numeric codegen path
- [ ] Evaluate building native CAS only if SymPy bottlenecks

### PhysicsMath — JIT kernels
- [x] Kernel cache: compile repeated tensor ops once, reuse (stub)
- [ ] `@kernel` attribute for hot loops → LLVM JIT
- [ ] CUDA kernel codegen stub (Phase 2 end)

### Standard library — math
- [x] `stdlib/math.phys` — LA, FFT, special functions
- [x] `stdlib/constants.phys` — CODATA physical constants with units

### Implicit parallelism
- [ ] MIR analysis: identify embarrassingly parallel `for` loops
- [ ] OpenMP codegen for CPU parallel loops
- [ ] `@parallel` override attribute
- [ ] `@gpu` attribute stub → CUDA codegen hook

### Legacy bridge v0
- [x] `extern fortran` FFI attribute (parser)
- [x] `extern c` FFI attribute
- [x] `bindgen` for calling existing `.so` / `.dll` libraries (`physlang-interop` + C)
- [x] Example: call legacy Fortran LAPACK routine from `.phys` (`examples/math/extern_daxpy.phys`)

### PhysicsSim — module stubs (interfaces only)
- [x] `physlang-sim` crate skeleton
- [x] Trait definitions: `Simulator`, `Mesh`, `BoundaryCondition`, `Solver`
- [x] Sub-module stubs: `continuum`, `particle`, `montecarlo`, `em`
- [x] No full solvers yet — API design + empty impls

### Phase 2 exit criteria
- [x] Dense LA benchmarks pass
- [x] Fortran extern call demonstrated
- [x] SymPy symbolic diff via FFI works

---

## Phase 3 — IDE shell + PhysicsViz + PhysicsAtom (months 6–9)

**Goal:** Unified hub experience; molecular/atomic tooling; full visualization.

### PhysicsLang IDE — unified shell
- [x] Tauri (or Electron) desktop app shell
- [x] Monaco editor embedded with `.phys` support
- [ ] Notebook interface: `.phys` cells + Python cells mixed
- [x] Project explorer: `.phys` files, stdlib, examples
- [~] Integrated terminal
- [ ] Debugger stub: breakpoints, variable inspect (LSP DAP)
- [ ] Job manager panel: queue, status, logs for long simulations
- [ ] Package hub: browse/install physics packages (registry stub)

### LSP — full feature set
- [~] Autocomplete for stdlib, user symbols
- [ ] Rename symbol
- [ ] Find references
- [ ] Code actions: import missing unit, fix dimension error
- [~] Formatting (`phys fmt`)

### PhysicsViz — 3D engine
- [~] wgpu + winit rendering backend
- [~] 3D field renderer: scalar fields (heatmaps), vector fields (arrows)
- [ ] Isosurface extraction (marching cubes)
- [ ] Volume rendering stub
- [ ] Camera controls: orbit, pan, zoom
- [ ] Time-series playback: animate simulation frames
- [ ] Export: PNG, MP4, VTK

### PhysicsViz — molecular viewer
- [~] Parse PDB, CIF, XYZ file formats
- [~] Ball-and-stick, space-filling, ribbon render modes
- [ ] Atom picking, bond highlighting
- [ ] Supercell / unit cell visualization

### PhysicsViz — 2D plots
- [ ] Live charts: line, scatter, histogram, contour
- [ ] Real-time streaming from running simulation
- [ ] Dashboard layout system (grid of plots)

### PhysicsAtom — molecular & atomic layer
- [x] `physlang-atom` crate
- [x] `stdlib/atom.phys` — atom, bond, molecule types
- [~] Periodic table data (atomic number, mass, radius, electronegativity)
- [ ] Orbital viewer: s, p, d orbital shapes (hydrogen-like)
- [ ] Bond types: single, double, triple, aromatic
- [ ] Bond order, length, angle calculations
- [ ] Force fields: Lennard-Jones, harmonic bonds/angles (basic MM)
- [ ] Open Babel FFI for format conversion and sanitization
- [ ] SMILES / InChI parsing via Open Babel
- [x] Molecular graph data structure (atoms + adjacency)

### PhysicsAtom — quantum chemistry (classical, not quantum computing)
- [ ] Hartree-Fock stub (integrate PySCF via Python FFI)
- [ ] DFT interface stub (call external codes: ORCA, Gaussian file prep)
- [ ] Basis set library: STO-3G, 6-31G* (data files)
- [ ] Electron density isosurface visualization hook

### Phase 3 exit criteria
- [x] IDE opens project, shows inline unit errors
- [x] Molecule viewer loads PDB and renders
- [~] 3D scalar field renders from simulation output

---

## Phase 4 — PhysicsSim multi-domain simulation (months 9–18)

**Goal:** PDE-native syntax; continuum, particle, Monte Carlo, EM domains.

### PDE-native language surface
- [ ] Unicode/math syntax: `∇²φ = ρ/ε₀`, `∂u/∂t = α∇²u`
- [ ] Domain annotation: `@domain(x in [0,1], y in [0,1])`
- [ ] Boundary conditions: `@bc(dirichlet)`, `@bc(neumann)`, `@bc(periodic)`
- [ ] Initial conditions: `@ic(u = 0)`
- [ ] `@discretize(fem|fdm|fdtd|spectral)` compiler attribute
- [ ] Compiler selects discretization + solver from PDE + domain + BCs
- [ ] Automatic mesh generation stub (Delaunay via Triangle lib)

### PhysicsSim — continuum (CFD / FEM / FDM)
- [ ] `stdlib/sim/continuum.phys`
- [ ] Mesh types: structured, unstructured, adaptive (stub)
- [ ] FEM: P1/P2 elements, assembly from weak form
- [ ] FDM: staggered grids for CFD
- [ ] Poisson, heat, advection-diffusion, Navier-Stokes (laminar stub)
- [ ] Boundary layer mesh refinement hook
- [ ] OpenFOAM case import (mesh + BCs; run via interop)
- [ ] COMSOL `.mph` export/import stub

### PhysicsSim — fluid dynamics (CFD focus)
- [ ] Incompressible Navier-Stokes solver (FVM or FEM)
- [ ] Turbulence models: k-ε, k-ω (stub)
- [ ] Multiphase flow stub (VOF)
- [ ] Compressible flow stub (Euler equations)
- [ ] Streamline, vorticity, pressure field visualization hooks
- [ ] Example: lid-driven cavity, cylinder flow

### PhysicsSim — particle methods
- [ ] `stdlib/sim/particle.phys`
- [ ] N-body gravitational/electrostatic simulation
- [ ] Molecular dynamics (MD): velocity Verlet integrator
- [ ] Force computation from PhysicsAtom force fields
- [ ] Periodic boundary conditions for MD
- [ ] SPH (Smoothed Particle Hydrodynamics) stub
- [ ] Neighbor list / cell linked list for O(N) forces
- [ ] Example: Lennard-Jones fluid MD

### PhysicsSim — Monte Carlo
- [ ] `stdlib/sim/montecarlo.phys`
- [ ] Metropolis-Hastings sampler
- [ ] Ising model simulation
- [ ] Statistical mechanics: partition function estimation
- [ ] Random number generators: reproducible seeds, parallel streams

### PhysicsSim — EM / optics / acoustics
- [ ] `stdlib/sim/em.phys`
- [ ] Maxwell equations FDTD (Yee grid)
- [ ] Wave equation FDM/FDTD
- [ ] Acoustic wave propagation
- [ ] PML (perfectly matched layer) absorbing BCs
- [ ] Example: 2D electromagnetic wave scattering

### Distributed simulation
- [ ] `@distributed` attribute → MPI codegen
- [ ] Domain decomposition for PDEs
- [ ] Halo exchange for ghost cells
- [ ] Checkpoint / restart: save simulation state to disk
- [ ] Fault tolerance stub (re-submit failed ranks)
- [ ] Job splitting for parameter sweeps

### Phase 4 exit criteria
- [ ] 2D Poisson PDE in ~10 lines of `.phys`, auto-FEM solves correctly
- [ ] Lid-driven cavity CFD example runs and visualizes
- [ ] MD simulation of 1000 particles completes with correct energy drift

---

## Phase 5 — PhysicsLab instrumentation extension (months 12–18)

**Goal:** LabVIEW-style hardware instrumentation as an opt-in IDE module.

### PhysicsLab — IDE module shell
- [ ] Separate extension/plugin in IDE (not core compiler)
- [ ] Visual dataflow editor: node graph canvas
- [ ] Node types: source, sink, transform, display, trigger
- [ ] Wire connections with type checking (voltage, current, digital)
- [ ] Save/load flow graphs as `.physlab` project files
- [ ] Deploy flow: compile graph → running server process

### PhysicsLab — data acquisition
- [ ] NI-DAQmx adapter (via Python `nidaqmx` FFI)
- [ ] PyVISA adapter: GPIB, USB, Ethernet instruments
- [ ] Serial port adapter (RS-232 / RS-485)
- [ ] TCP/UDP socket source nodes
- [ ] File replay source (CSV, HDF5, TDMS)
- [ ] Sample rate, buffer size, channel config UI

### PhysicsLab — signal processing
- [ ] `.phys` kernels for DSP: FFT, filter (low/high/bandpass), RMS, peak detect
- [ ] Lock-in amplifier simulation node
- [ ] PID controller node
- [ ] Dataflow → call compiled `.phys` signal processing fns in real time

### PhysicsLab — live dashboards
- [ ] Widget library: gauge, chart, numeric display, LED, toggle, slider
- [ ] Bind widget to dataflow node output
- [ ] Dashboard layout editor (drag-and-drop)
- [ ] Web dashboard export (local HTTP server stub)

### PhysicsLab — hardware targets
- [ ] Arduino: upload sketch template from flow graph
- [ ] FPGA: pre-built bitstream templates (not custom HDL compiler)
- [ ] Raspberry Pi GPIO stub
- [ ] SCPI command generator for programmable instruments

### PhysicsLab — server mode
- [ ] Long-running acquisition server process
- [ ] REST API for reading channels / writing setpoints
- [ ] WebSocket streaming for live dashboard clients
- [ ] Logging and alarm thresholds

### Phase 5 exit criteria
- [ ] Live plot from USB instrument via dataflow graph
- [ ] Dashboard updates in real time from DAQ source
- [ ] `.physlab` project saves and reloads correctly

---

## Phase 6 — Interop & legacy bridge (ongoing, parallel)

**Goal:** Adapt to existing physics software without rewriting.

### Language interop
- [ ] Python: full bidirectional FFI (already Phase 1; extend)
- [ ] NumPy array interop (zero-copy)
- [ ] SciPy call wrappers
- [ ] Qiskit, PennyLane, PySCF, SymPy (quantum/chemistry)
- [ ] C / C++ `extern` blocks with header parsing
- [ ] Fortran `extern fortran` with name mangling
- [ ] MATLAB `.mat` file read/write (via scipy.io or matio)
- [ ] Julia call stub (future)

### File format interop
- [ ] VTK, VTU, XDMF mesh + field I/O
- [ ] HDF5 scientific datasets
- [ ] NetCDF (climate/ocean data)
- [ ] OpenFOAM case directory import
- [ ] COMSOL `.mph` / `.txt` export import
- [ ] Gaussian `.fchk`, `.cube` (chemistry)
- [ ] PDB, CIF, XYZ (molecular)

### Package ecosystem
- [ ] Package registry design (physlang packages)
- [ ] `phys pkg init`, `phys pkg publish`, `phys pkg install`
- [ ] Version pinning and dependency resolution
- [ ] Community package template

---

## Phase 7 — Hardware backends & performance (ongoing)

**Goal:** Run everywhere physics runs — CPU, GPU, clusters, WASM, FPGA.

### LLVM targets
- [ ] CPU: x86_64, aarch64 with SIMD (AVX2/AVX-512, NEON)
- [ ] CUDA backend for NVIDIA GPUs
- [ ] Metal backend for Apple GPUs
- [ ] WASM backend for browser / Jupyter WASM notebooks
- [ ] Cross-compilation toolchain docs

### GPU & parallelism
- [ ] `@gpu` kernel codegen (CUDA + Metal)
- [ ] Automatic GPU offload for eligible loops
- [ ] Multi-GPU stub
- [ ] MPI cluster codegen from `@distributed`

### Performance tooling
- [ ] Built-in profiler: `phys profile run.phys`
- [ ] Flamegraph output
- [ ] Memory usage report
- [ ] Compiler optimization levels: `-O0`, `-O1`, `-O2`, `-O3`

---

## Phase 8 — PhysicsOS (deferred / research)

> Not planned for near-term. Language adoption does not require a custom OS.

- [ ] Research note: when / if PhysicsOS makes sense
- [ ] NUMA-aware scheduling concept doc
- [ ] RDMA / HPC cluster integration concept doc

---

## Cross-cutting concerns (all phases)

### Documentation
- [ ] Language reference manual
- [ ] Standard library docs (auto-generated from `.phys` doc comments)
- [ ] Tutorials: quantum, CFD, MD, lab instrumentation
- [ ] Migration guides: from Python, Julia, Fortran

### Testing
- [ ] Unit tests per crate (`cargo test`)
- [ ] Integration tests: compile + run examples
- [ ] Snapshot tests for compiler diagnostics
- [ ] Benchmark suite (Criterion.rs)
- [ ] Physical correctness tests (analytical solutions vs numerical)

### Security & packaging
- [ ] Sandboxed `extern` calls (optional)
- [ ] Signed package registry (future)
- [ ] Reproducible builds

---

## Quick reference — 9-layer architecture

| Layer | Crate / path | Phase |
|-------|-------------|-------|
| 1. PhysicsLang IDE | `apps/ide/`, `extensions/vscode-physlang/` | 0, 3 |
| 2. Compiler core | `physlang/physlang-{parser,types,mir,llvm}/` | 0, 1, 2 |
| 3. PhysicsMath | `physlang/physlang-runtime/`, `stdlib/math.phys` | 2 |
| 4. PhysicsSim | `physlang/physlang-sim/`, `stdlib/sim/` | 2 stub, 4 full |
| 5. PhysicsAtom | `physlang/physlang-atom/`, `stdlib/atom.phys` | 1 quantum, 3 molecular |
| 6. PhysicsViz | `physlang/physlang-viz/` | 1 minimal, 3 full |
| 7. PhysicsLab | `extensions/physicslab/` | 5 |
| 8. Interop & legacy | `physlang/physlang-interop/`, `bindings/` | 1 Python, 2 Fortran, 6 full |
| 9. Hardware targets | `physlang/physlang-llvm/` backends | 0 CPU, 2 CUDA stub, 7 full |

---

## Current focus

**Active phase:** Phase 3 — IDE shell + PhysicsViz + PhysicsAtom  
**Next actionable items:**
1. PhysicsViz: PDB parser + wgpu 3D backend (replace canvas stub)
2. LSP: rename, find references, user-symbol completion
3. PhysicsAtom: expand periodic table; Open Babel FFI stub

---

*Last updated: 2026-06-08. Update this file as items are completed.*
