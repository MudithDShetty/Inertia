# PhysicsLang language reference

> **Version:** Phase 3 (2026) · **Extension:** `.phys`  
> This document describes syntax and semantics **implemented today**. Features marked *planned* are on the roadmap in [Todo.md](../Todo.md).

---

## Table of contents

1. [Overview](#overview)
2. [Hello physics](#hello-physics)
3. [Lexical structure](#lexical-structure)
4. [Types](#types)
5. [SI units and quantities](#si-units-and-quantities)
6. [Functions and variables](#functions-and-variables)
7. [Expressions and operators](#expressions-and-operators)
8. [Quantum computing](#quantum-computing)
9. [Attributes and FFI](#attributes-and-ffi)
10. [Standard library](#standard-library)
11. [Command-line tools](#command-line-tools)
12. [IDE support](#ide-support)
13. [Not yet implemented](#not-yet-implemented)

---

## Overview

PhysicsLang is designed for computational physics workflows:

- **Compile-time SI checking** — catch `Force + Velocity`-style errors before run time  
- **First-class quantum syntax** — gates, registers, Hamiltonians, expectation values  
- **Native + Python hybrid** — hot paths in `.phys`, libraries via `@python.import`  
- **Single artifact** — one `.phys` file can be checked, interpreted, or compiled to native code (LLVM path)  

A program is a **module**: a sequence of top-level items (functions, quantum registers, constants, extern declarations).

---

## Hello physics

```phys
// Comments start with //

fn main() -> Int {
    let v: Velocity = 9.8 m/s      // unit literal
    let m: Mass = 1.0 kg
    let ke: Energy = 0.5 * m * v * v   // dimensions checked
    return 0
}
```

Every runnable program needs a **`main` function**. Its return type is usually `Int` (exit code) or a physics type such as `Energy` for quantum demos.

---

## Lexical structure

### Identifiers

Letters, digits, and underscores; must not start with a digit.

```phys
energy   theta   q0   my_hamiltonian
```

### Keywords

| Keyword | Purpose |
|---------|---------|
| `fn` | Define a function |
| `let` | Bind a variable (local or top-level) |
| `return` | Return from a function |
| `qreg` | Declare a quantum register |
| `extern` | Foreign (C/Fortran) function declaration |

*Reserved for future use:* `if`, `else` (recognized by the lexer; statement form not yet parsed).

### Literals

| Form | Example | Type |
|------|---------|------|
| Integer | `42`, `-1` | `Int` |
| Float | `3.14`, `1e-8`, `-0.5` | `Float` |
| String | `"hello"` | `String` |
| Quantity | `9.8 m/s`, `1.054571817e-34 J*s` | SI-checked quantity |
| Boolean | *planned* | `Bool` |

### Comments

```phys
// Line comment to end of line
```

### File layout

```phys
// 1. Optional top-level constants
let hbar: Float = 1.054571817e-34

// 2. Quantum register (if needed)
qreg q[2]

// 3. Functions
fn helper(x: Float) -> Float { return x }

// 4. Entry point
fn main() -> Int { return 0 }
```

---

## Types

### Scalar types

| Type | Description |
|------|-------------|
| `Int` | Signed integer |
| `Float` | IEEE double-precision |
| `Bool` | Boolean (*type exists; literal syntax limited*) |
| `String` | UTF-8 string |
| `Void` | No value (rare as return) |

### SI named types

These are **dimension-tagged** types. The compiler verifies that operations preserve SI dimensions.

| Type | SI dimensions | Example literal |
|------|---------------|-----------------|
| `Mass` | `[M]` | `1.0 kg` |
| `Velocity` | `[L/T]` | `9.8 m/s` |
| `Force` | `[M·L/T²]` | `10.0 N` |
| `Energy` | `[M·L²/T²]` | `1.0 J` |
| `Action` | `[M·L²/T]` | `hbar` in J·s |
| `Angle` | dimensionless (radians) | `0.1` or param vector |

### Quantum types

| Type | Description |
|------|-------------|
| `Qubit` | Single qubit (conceptual) |
| `QReg` | Register declared with `qreg q[n]` |
| `Gate` | Quantum gate operation |
| `Circuit` | Composed gate sequence |
| `Hamiltonian` | Observable / Hamiltonian operator |
| `Observable` | Hermitian observable for `expect` |
| `Result` | Measurement / sampling result |

### Array types

Fixed-size parameter vectors use bracket syntax:

```phys
fn energy(theta: Angle[4]) -> Energy { ... }
fn qaoa(params: Angle[2]) -> Energy { ... }
```

`Angle[n]` denotes `n` variational parameters (typically one float per angle, stored as a flat vector at runtime).

### Type annotations

```phys
let x: Float = 1.0
let v: Velocity = 5.0 m/s
fn f(n: Int) -> Float { return 0.0 }
```

Omitting the annotation on `let` is allowed when the initializer supplies a inferrable type:

```phys
let x = 1.0          // Float
let H = Z(0) @ Z(1)   // Hamiltonian (from expression context)
```

---

## SI units and quantities

### Unit literals

Attach SI units directly to numeric literals:

```phys
let c: Float = 299792458.0        // unitless float
let v: Velocity = 9.8 m/s
let F: Force = 100.0 N
let E: Energy = 1.602e-19 J
let p: Action = 1.054571817e-34 J*s
```

### Unit algebra in literals

| Syntax | Meaning |
|--------|---------|
| `m/s` | metres per second |
| `m*s` or `m s` | product (implicit multiply) |
| `J*s` | joule-seconds |
| `m^2` | square metres (`^` power) |
| `kg*m/s^2` | equivalent to newtons |

### Supported unit symbols

| Symbol | Name | SI base |
|--------|------|---------|
| `m`, `km` | length | m |
| `kg`, `g` | mass | kg |
| `s` | time | s |
| `A` | current | A |
| `K` | temperature | K |
| `mol` | amount | mol |
| `cd` | luminous intensity | cd |
| `N` | newton | derived |
| `J` | joule | derived |
| `Pa` | pascal | derived |
| `W` | watt | derived |

### Dimension errors

The compiler rejects incompatible assignments and binary operations:

```phys
// ERROR: dimension mismatch
let bad: Force = 1.0 J

// ERROR: cannot add Mass + Velocity
let x: Mass = 1.0 kg
let y: Velocity = 1.0 m/s
let z = x + y
```

Error messages include **line and column** (shown in the IDE and `phys check`).

---

## Functions and variables

### Function definition

```phys
fn name(param1: Type1, param2: Type2) -> ReturnType {
    // body
    return value
}
```

Example:

```phys
fn kinetic(m: Mass, v: Velocity) -> Energy {
    return 0.5 * m * v * v
}
```

### Top-level `let`

Module-level constants:

```phys
let H = -0.39 * X(0) @ X(1) + 0.18 * Z(0) + 0.18 * Z(1)
```

### Local `let`

Inside a function body:

```phys
fn main() -> Int {
    let x: Float = 3.0
    let y = x * 2.0
    return 0
}
```

### Return

```phys
return expression
return                  // void return in void functions
```

### Entry point

```phys
fn main() -> Int {
    return 0            // exit code 0 = success
}
```

---

## Expressions and operators

### Arithmetic

| Operator | Meaning | SI note |
|----------|---------|---------|
| `+` `-` | add, subtract | Operands must match dimensions |
| `*` `/` | multiply, divide | Dimensions combine algebraically |
| unary `-` | negation | |

### Comparison

| Operator | Meaning |
|----------|---------|
| `==` `!=` | equality |
| `<` `<=` `>` `>=` | ordering (scalars) |

### Logical

| Operator | Meaning |
|----------|---------|
| `&&` | logical and |
| `\|\|` | logical or |
| `!` | logical not |

### Tensor product (quantum)

The `@` operator builds tensor products of gates and Hamiltonian terms:

```phys
let term = X(0) @ X(1)
let H = Z(0) @ Z(1) + X(0)
```

### Function calls

```phys
let e = kinetic(1.0 kg, 10.0 m/s)
let val = abs(-3.0)
```

### Gate calls

See [Quantum computing](#quantum-computing) — gates use the same call syntax: `H(0)`, `CNOT(0, 1)`.

---

## Quantum computing

### Quantum register

Declare before using qubit indices in gates:

```phys
qreg q[2]     // two qubits, indices 0 and 1
```

The name `q` is conventional; the register size must cover all qubit indices used.

### Built-in gates

| Gate | Syntax | Description |
|------|--------|-------------|
| Hadamard | `H(q)` | Superposition |
| Pauli X | `X(q)` | NOT |
| Pauli Y | `Y(q)` | |
| Pauli Z | `Z(q)` | |
| S | `S(q)` | Phase π/2 |
| T | `T(q)` | Phase π/4 |
| CNOT | `CNOT(c, t)` | Controlled-NOT |
| CZ | `CZ(c, t)` | Controlled-Z |
| SWAP | `SWAP(a, b)` | Swap qubits |
| RX | `RX(θ, q)` | X rotation |
| RY | `RY(θ, q)` | Y rotation |
| RZ | `RZ(θ, q)` | Z rotation |
| U3 | `U3(θ, φ, λ, q)` | General single-qubit |

**Qubit indices** are non-negative integers: `H(0)`, `CNOT(0, 1)`.

**Parameters** (`θ`, `φ`) are `Float` or `Angle` expressions; for parametric ansätze pass the parameter vector:

```phys
RY(theta, 0)    // theta is Angle[n] or Float
RZ(params, 1)
```

### Building circuits

Gate calls in a function body append to the implicit circuit builder. Return a `Circuit` via helpers:

```phys
fn bell() -> Circuit {
    H(0)
    CNOT(0, 1)
    return ansatz(q, 0.0)
}
```

| Function | Purpose |
|----------|---------|
| `ansatz(q, params)` | Wrap current gate sequence as a `Circuit` |
| `expect(circuit, H)` | Expectation value ⟨ψ\|H\|ψ⟩ → `Energy` |
| `sample(circuit, shots)` | Shot histogram → `Result` |

### Hamiltonians

Build from Pauli terms with `@` and arithmetic:

```phys
let H = -0.39 * X(0) @ X(1) + 0.18 * Z(0) + 0.18 * Z(1)
```

Observables for single-qubit terms:

```phys
let obs = Z(0)
let two = Z(0) @ Z(1)
```

### Variational algorithms

Mark energy functions with `@differentiable` for autodiff (parameter-shift / adjoint):

```phys
@differentiable
fn energy(theta: Angle[4]) -> Energy {
    RY(theta, 0)
    RY(theta, 1)
    CNOT(0, 1)
    let circuit = ansatz(q, theta)
    return expect(circuit, H)
}
```

### Complete VQE example

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

Run: `phys run examples/quantum/h2_vqe.phys`

### QAOA pattern

```phys
qreg q[2]

@differentiable
fn qaoa_energy(params: Angle[2]) -> Energy {
    RZ(params, 0)
    RZ(params, 1)
    RX(params, 0)
    CNOT(0, 1)
    let H = Z(0) @ Z(1)
    let circuit = ansatz(q, params)
    return expect(circuit, H)
}
```

### Sampling

```phys
fn main() -> Int {
    let circ = grover_circuit()
    let result = sample(circ, 1024)
    return 0
}
```

---

## Attributes and FFI

Attributes use `@name` or `@name(args)` before a function or extern.

| Attribute | Status | Purpose |
|-----------|--------|---------|
| `@differentiable` | **Implemented** | Enable reverse-mode autodiff through the function |
| `@python.import("module")` | **Implemented** | Call Python from generated / interpreted code |
| `@gpu` | Stub | Future CUDA/Metal kernel |
| `@parallel` | Stub | Future OpenMP parallel loop |

Example Python FFI (see `examples/quantum/qiskit_ffi.phys`):

```phys
@python.import("qiskit")
fn export_circuit(c: Circuit) -> Int {
    return 0
}
```

### C / Fortran extern

Declare legacy library entry points:

```phys
extern c fn daxpy(alpha: Float, x: Float, y: Float) -> Float
```

Execution uses the interop layer (`physlang-interop`); see `examples/math/extern_daxpy.phys`.

---

## Standard library

Standard library modules live in `stdlib/` as `.phys` files. They are referenced by convention (full `import` syntax is *planned*).

| File | Contents |
|------|----------|
| `stdlib/core.phys` | `c_light`, `g_std`, `hbar_si`, `abs` |
| `stdlib/constants.phys` | CODATA 2018 constants |
| `stdlib/quantum.phys` | `bell`, `h2_hamiltonian`, `vqe_energy`, `run_shots` |
| `stdlib/math.phys` | `dot`, `norm2`; LA/FFT via FFI |
| `stdlib/atom.phys` | Molecular types (*stub*) |

### Quantum helpers (`stdlib/quantum.phys`)

```phys
fn bell() -> Circuit
fn h2_hamiltonian() -> Hamiltonian
fn hardware_efficient_ansatz(n: Int, layers: Int, params: Angle) -> Circuit

@differentiable
fn vqe_energy(theta: Angle[4]) -> Energy

fn run_shots(circ: Circuit, shots: Int) -> Result
```

---

## Command-line tools

```bash
phys run FILE [--entry NAME]     # default entry: main
phys check FILE                   # type + unit check
phys build FILE [--emit ast|mir|llvm] [--target cpu]
phys viz FILE -o diagram.svg      # circuit SVG
phys lsp FILE                     # diagnostics JSON
phys repl                         # interactive stub
phys fmt FILE                     # format stub
```

From Cargo without installing:

```bash
cargo run -- run examples/hello.phys
cargo run -- check examples/quantum/grover.phys
```

---

## IDE support

The **PhysicsLang IDE** (`apps/ide/`) provides:

| Feature | Shortcut / UI |
|---------|----------------|
| Syntax highlighting | `.phys` Monaco grammar |
| Type / unit diagnostics | **Check** or on save |
| Completion | Ctrl+Space |
| Hover docs | Mouse over gates / types |
| Go to definition | **F12** |
| Find references | **Shift+F12** |
| Rename symbol | **F2** |
| Run program | **Run** |
| Language docs | **Docs** toolbar |
| Molecule / field viewer | Open `.gjf`, `.pdb`, `.fchk`, `.cube` |
| Chem jobs | **Run G16** / **Run ORCA** + Jobs panel |

---

## Not yet implemented

The following appear in roadmap / lexer but are **not** available in programs today:

| Feature | Status |
|---------|--------|
| `if` / `else` statements | Lexer only |
| `for` / `while` loops | Not parsed |
| `struct` / `enum` user types | Not parsed (`stdlib/atom.phys` is aspirational) |
| Module `import` | Not parsed |
| Compile-time `km` → `m` conversion | Planned |
| PDE unicode syntax (`∇²`, `∂/∂t`) | Phase 4 |
| Full `phys fmt` | Stub |

When in doubt, run **`phys check yourfile.phys`** or open the file in the IDE — if it parses and checks, the syntax is supported.

---

## Related material

- [Quick start](quickstart.md)  
- [Architecture](architecture.md)  
- [Todo.md](../Todo.md) — full roadmap  
- [examples/](../examples/) — runnable programs  
