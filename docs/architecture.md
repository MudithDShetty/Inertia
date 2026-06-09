# Inertia Architecture

Inertia is a standalone `.phys` compiled language with Python FFI, organized as a nine-layer stack.

## Layer diagram

```mermaid
flowchart TB
  subgraph L1 [1_Inertia_IDE]
    Editor[Editor_LSP]
    Notebook[Notebook]
    JobMgr[Job_Manager]
  end

  subgraph L2 [2_Compiler_Core]
    Parser[Parser_TypeChecker]
    Units[SI_Unit_System]
    Autodiff[Native_Autodiff]
    ParInf[Parallelism_Inference]
    LLVM[LLVM_Backend]
  end

  subgraph L3 [3_PhysicsMath]
    LA[Linear_Algebra]
    FFT[FFT_CAS]
    JIT[JIT_Kernels]
  end

  subgraph L4 [4_PhysicsSim]
    CFD[CFD_FEM_FDM]
    MD[MD_Nbody_SPH]
    MC[Monte_Carlo]
    EM[EM_FDTD]
  end

  subgraph L5 [5_PhysicsAtom]
    Atoms[Atoms_Bonds]
    QChem[DFT_QChem]
  end

  subgraph L6 [6_PhysicsViz]
    Field3D[3D_Fields]
    Molecule[Molecule_Viewer]
    Plots[Live_Plots]
  end

  subgraph L7 [7_PhysicsLab]
    Dataflow[Dataflow_Editor]
    DAQ[DAQ_GPIB_USB]
    Dashboard[Dashboards]
  end

  subgraph L8 [8_Interop]
    Legacy[Fortran_C_MATLAB]
    Formats[OpenFOAM_COMSOL]
  end

  subgraph L9 [9_Hardware]
    CPU[CPU_x86_ARM]
    GPU[CUDA_Metal]
    Cluster[MPI_FPGA]
  end

  L1 --> L2
  L2 --> L3
  L2 --> L4
  L2 --> L5
  L2 --> L6
  L1 --> L7
  L3 --> L4
  L4 --> L6
  L5 --> L6
  L2 --> L8
  L8 --> Python[Python_NumPy_Qiskit]
  L2 --> L9
```

## Compiler pipeline

```
.phys source
  → Lexer + Parser (physlang-parser)
  → Type check + SI units (physlang-types)
  → MIR + autodiff (physlang-mir)
  → LLVM IR (physlang-llvm) OR Interpreter (physlang-runtime)
```

## Crate map

| Crate | Layer | Status |
|-------|-------|--------|
| `physlang-parser` | Compiler | Phase 0 |
| `physlang-types` | Compiler | Phase 0 |
| `physlang-mir` | Compiler | Phase 0–1 |
| `physlang-llvm` | Compiler | Phase 0 (pseudo-IR) |
| `physlang-runtime` | Math/runtime | Phase 0–1 |
| `physlang-quantum` | Atom/quantum | Phase 1 |
| `physlang-viz` | Viz | Phase 1 |
| `physlang-lsp` | IDE | Phase 1 |
| `physlang-cli` | IDE | Phase 0 |
| `physlang-py` | Interop | Phase 1 |

## Design decisions

- **Standalone `.phys`** with Python FFI (not embedded DSL)
- **First MVP vertical:** quantum computing (Qiskit / PennyLane)
- **PhysicsLab** is a separate IDE extension (Phase 5)
- **PhysicsOS** deferred indefinitely

See [Todo.md](../Todo.md) for the full implementation roadmap.
