# Inertia standard library

Auto-generated from `stdlib/*.phys` line comments (`//` above each symbol).

Regenerate: `.\scripts\gen-stdlib-docs.ps1`

See also [language reference](language-reference.md#standard-library).

## `atom.phys`

PhysicsAtom standard library — molecular & atomic types (Phase 3 stub)
Backed by physlang-atom crate (periodic table, bond graph).

### `Atom` (let)

Single atom with Cartesian coordinates (Å or Bohr per context)

```phys
struct Atom {
```

### `Bond` (let)

Bond between two atom indices

```phys
struct Bond {
```

### `Molecule` (let)

Molecular graph container

```phys
struct Molecule {
```

### `element_symbol` (fn)

Element symbol from atomic number (native stub)

```phys
fn element_symbol(z: Int) -> String {
```

### `bond_length` (fn)

Bond length in current length units

```phys
fn bond_length(mol: Molecule, bond: Bond) -> Float {
```

## `constants.phys`

CODATA 2018 physical constants (SI) — see also stdlib/core.phys

### `c` (let)

```phys
let c: Float = 299792458.0              // speed of light (m/s)
```

### `h` (let)

```phys
let h: Float = 6.62607015e-34           // Planck constant (J·s)
```

### `hbar` (let)

```phys
let hbar: Float = 1.054571817e-34       // reduced Planck (J·s)
```

### `e` (let)

```phys
let e: Float = 1.602176634e-19          // elementary charge (C)
```

### `m_e` (let)

```phys
let m_e: Float = 9.1093837015e-31       // electron mass (kg)
```

### `m_p` (let)

```phys
let m_p: Float = 1.67262192369e-27      // proton mass (kg)
```

### `k_B` (let)

```phys
let k_B: Float = 1.380649e-23           // Boltzmann constant (J/K)
```

### `N_A` (let)

```phys
let N_A: Float = 6.02214076e23          // Avogadro constant (1/mol)
```

### `G` (let)

```phys
let G: Float = 6.67430e-11              // gravitational constant (m³/kg/s²)
```

### `mu_0` (let)

```phys
let mu_0: Float = 1.25663706212e-6      // vacuum permeability (N/A²)
```

### `epsilon_0` (let)

```phys
let epsilon_0: Float = 8.8541878128e-12  // vacuum permittivity (F/m)
```

### `a_0` (let)

```phys
let a_0: Float = 5.29177210903e-11      // Bohr radius (m)
```

### `E_h` (let)

```phys
let E_h: Float = 4.3597447222071e-18    // Hartree energy (J)
```

## `core.phys`

Inertia core standard library — unit constants and basic math

### `c_light` (let)

Speed of light in vacuum (m/s)

```phys
let c_light: Float = 299792458.0
```

### `g_std` (let)

Standard gravity (m/s²)

```phys
let g_std: Float = 9.80665
```

### `hbar_si` (let)

Reduced Planck constant (J·s)

```phys
let hbar_si: Float = 1.054571817e-34
```

### `e_charge` (let)

Elementary charge (C)

```phys
let e_charge: Float = 1.602176634e-19
```

### `abs` (fn)

Absolute value (identity stub for interpreter)

```phys
fn abs(x: Float) -> Float {
```

## `math.phys`

Inertia math standard library — interfaces for PhysicsMath engine

### `dot` (fn)

Dot product stub (scalar multiply placeholder)

```phys
fn dot(a: Float, b: Float) -> Float {
```

### `norm2` (fn)

Squared L2 norm stub

```phys
fn norm2(v: Float) -> Float {
```

## `quantum.phys`

Inertia quantum standard library — gates, circuits, Hamiltonians, VQA helpers

### `bell` (fn)

Bell-state preparation circuit (H on q0, CNOT q0→q1)

```phys
fn bell() -> Circuit {
```

### `hardware_efficient_ansatz` (fn)

Hardware-efficient layered ansatz for n qubits

```phys
fn hardware_efficient_ansatz(n: Int, layers: Int, params: Angle) -> Circuit {
```

### `pauli_z` (fn)

Pauli-Z observable on one qubit

```phys
fn pauli_z(qubit: Int) -> Observable {
```

### `heisenberg_hamiltonian` (fn)

Heisenberg XXX chain Hamiltonian on two qubits

```phys
fn heisenberg_hamiltonian(j: Float) -> Hamiltonian {
```

### `h2_hamiltonian` (fn)

H₂ stoquastic Hamiltonian (reference for VQE examples)

```phys
fn h2_hamiltonian() -> Hamiltonian {
```

### `vqe_energy` (fn)

VQE energy expectation ⟨H⟩ for hardware-efficient ansatz

```phys
fn vqe_energy(theta: Angle[4]) -> Energy {
```

### `grover_oracle` (fn)

Grover oracle phase flip (example stub)

```phys
fn grover_oracle() -> Circuit {
```

### `run_shots` (fn)

Sample measurement outcomes from a circuit

```phys
fn run_shots(circ: Circuit, shots: Int) -> Result {
```
