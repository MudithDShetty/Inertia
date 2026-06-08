"""Qiskit Aer + PennyLane execution for PhysicsLang quantum programs."""

from __future__ import annotations

import json
from typing import Any

import numpy as np

# Fallback STO-3G H2 coefficients (Hartree) — overridden by PennyLane qchem when available
H2_PAULI_TERMS: list[tuple[str, float]] = [
    ("II", -0.011280),
    ("IZ", 0.171201),
    ("ZI", 0.171201),
    ("ZZ", 0.011280),
    ("XX", 0.045893),
    ("YY", 0.045893),
]

_H2_CACHE: dict[str, Any] | None = None


def _h2_geometry():
    return ["H", "H"], np.array([0.0, 0.0, -0.66140414, 0.0, 0.0, 0.66140414])


def _get_h2_context() -> dict[str, Any]:
    """Lazy-build H2 STO-3G Hamiltonian, UCC ansatz metadata, and reference energy."""
    global _H2_CACHE
    if _H2_CACHE is not None:
        return _H2_CACHE

    qml = _require_pennylane()
    symbols, coords = _h2_geometry()
    electrons = 2
    H_pl, n_qubits = qml.qchem.molecular_hamiltonian(
        symbols, coords, basis="sto-3g", active_electrons=electrons, active_orbitals=2
    )
    hf = qml.qchem.hf_state(electrons, n_qubits)
    singles, doubles = qml.qchem.excitations(electrons, n_qubits)
    ref = float(
        np.min(np.real(np.linalg.eigvalsh(qml.matrix(H_pl, wire_order=range(n_qubits)))))
    )

    from qiskit.quantum_info import Operator

    SparsePauliOp, *_ = _require_qiskit()
    mat = np.asarray(qml.matrix(H_pl, wire_order=range(n_qubits)), dtype=complex)
    mat = _permute_matrix_for_qiskit(mat, n_qubits)
    H_qiskit = SparsePauliOp.from_operator(Operator(mat))

    _H2_CACHE = {
        "H_pl": H_pl,
        "H_qiskit": H_qiskit,
        "ref": ref,
        "n_qubits": n_qubits,
        "hf": hf,
        "singles": singles,
        "doubles": doubles,
    }
    return _H2_CACHE


def _bit_reverse_permutation(n_qubits: int) -> np.ndarray:
    dim = 2**n_qubits
    perm = np.empty(dim, dtype=int)
    for i in range(dim):
        rev = 0
        for b in range(n_qubits):
            rev |= ((i >> b) & 1) << (n_qubits - 1 - b)
        perm[i] = rev
    return perm


def _permute_matrix_for_qiskit(matrix: np.ndarray, n_qubits: int) -> np.ndarray:
    """Map PennyLane wire-ordered operator matrix to Qiskit little-endian indexing."""
    perm = _bit_reverse_permutation(n_qubits)
    return matrix[np.ix_(perm, perm)]


def _build_h2_hamiltonians():
    ctx = _get_h2_context()
    return ctx["H_pl"], ctx["H_qiskit"], ctx["ref"], ctx["n_qubits"]


def _require_qiskit():
    try:
        from qiskit.quantum_info import SparsePauliOp
        from qiskit_aer.primitives import EstimatorV2 as AerEstimator
        from qiskit.circuit import QuantumCircuit, Parameter
        from qiskit import transpile
        from qiskit_aer import AerSimulator
    except ImportError as e:
        raise ImportError(
            "Install quantum extras: pip install physlang[quantum]"
        ) from e
    return SparsePauliOp, AerEstimator, QuantumCircuit, Parameter, transpile, AerSimulator


def _require_pennylane():
    try:
        import pennylane as qml
    except ImportError as e:
        raise ImportError(
            "Install quantum extras: pip install physlang[quantum]"
        ) from e
    return qml


def _pauli_string_to_op(s: str) -> Any:
    qml = _require_pennylane()
    ops = []
    for wire, c in enumerate(reversed(s)):
        if c == "X":
            ops.append(qml.PauliX(wire))
        elif c == "Y":
            ops.append(qml.PauliY(wire))
        elif c == "Z":
            ops.append(qml.PauliZ(wire))
    if not ops:
        return qml.Identity(wires=0)
    result = ops[0]
    for op in ops[1:]:
        result = result @ op
    return result


def h2_hamiltonian_qiskit():
    _, H_qiskit, _, _ = _build_h2_hamiltonians()
    return H_qiskit


def h2_hamiltonian_pennylane():
    H_pl, _, _, _ = _build_h2_hamiltonians()
    return H_pl


def h2_reference_energy() -> float:
    """Exact ground-state energy of H2 STO-3G (Ha)."""
    _, _, ref, _ = _build_h2_hamiltonians()
    return ref


def circuit_ir_to_qiskit(circuit_json: str | dict) -> Any:
    _, _, QuantumCircuit, _, _, _ = _require_qiskit()
    data = json.loads(circuit_json) if isinstance(circuit_json, str) else circuit_json
    n = int(data.get("num_qubits", 2))
    qc = QuantumCircuit(n, name=data.get("name", "physlang"))
    for gate in data.get("gates", []):
        name = gate["name"].upper()
        targets = gate.get("targets", [])
        params = gate.get("params", [])
        if name == "H":
            qc.h(targets[0])
        elif name == "X":
            qc.x(targets[0])
        elif name == "Y":
            qc.y(targets[0])
        elif name == "Z":
            qc.z(targets[0])
        elif name == "CNOT":
            qc.cx(targets[0], targets[1])
        elif name == "CZ":
            qc.cz(targets[0], targets[1])
        elif name == "SWAP":
            qc.swap(targets[0], targets[1])
        elif name in ("RX", "RY", "RZ"):
            getattr(qc, name.lower())(float(params[0]) if params else 0.0, targets[0])
    return qc


def build_h2_ansatz_qiskit(params: list[float] | np.ndarray) -> Any:
    """H2 UCCSD-style ansatz (4 qubits) matching PennyLane qchem conventions."""
    qml = _require_pennylane()
    _, _, QuantumCircuit, _, _, _ = _require_qiskit()
    ctx = _get_h2_context()
    p = np.asarray(params, dtype=float).flatten()
    if len(p) < 3:
        p = np.resize(p, 3)

    qc = QuantumCircuit(ctx["n_qubits"])
    for wire, bit in enumerate(ctx["hf"]):
        if bit:
            qc.x(wire)

    for idx, wires in enumerate(ctx["singles"]):
        for op in qml.SingleExcitation(float(p[idx]), wires=wires).decomposition():
            _apply_pl_op_to_qiskit(qc, op)

    for idx, wires in enumerate(ctx["doubles"]):
        for op in qml.DoubleExcitation(float(p[len(ctx["singles"]) + idx]), wires=wires).decomposition():
            _apply_pl_op_to_qiskit(qc, op)

    return qc


def _apply_pl_op_to_qiskit(qc: Any, op: Any) -> None:
    wires = [int(w) for w in op.wires]
    name = op.name
    if name in ("H", "Hadamard"):
        qc.h(wires[0])
    elif name == "PauliX":
        qc.x(wires[0])
    elif name == "CNOT":
        qc.cx(wires[0], wires[1])
    elif name == "RY":
        qc.ry(float(op.parameters[0]), wires[0])
    elif name == "RZ":
        qc.rz(float(op.parameters[0]), wires[0])
    elif name == "RX":
        qc.rx(float(op.parameters[0]), wires[0])
    else:
        raise ValueError(f"Unsupported PennyLane op for Qiskit export: {name}")


def run_qiskit(
    circuit_json: str | dict | None = None,
    *,
    params: list[float] | None = None,
    hamiltonian: str = "h2",
    backend: str = "aer_simulator",
) -> dict[str, Any]:
    (
        SparsePauliOp,
        AerEstimator,
        _,
        _,
        transpile,
        AerSimulator,
    ) = _require_qiskit()

    observable = h2_hamiltonian_qiskit() if hamiltonian == "h2" else SparsePauliOp.from_list(json.loads(hamiltonian))

    if circuit_json is not None:
        circuit = circuit_ir_to_qiskit(circuit_json)
    elif params is not None:
        circuit = build_h2_ansatz_qiskit(params)
    else:
        circuit = build_h2_ansatz_qiskit([0.0, 0.0, 0.0])

    sim = AerSimulator(method="statevector")
    isa_circuit = transpile(circuit, sim, optimization_level=1)
    estimator = AerEstimator(
        options={
            "default_precision": 1e-4,
            "backend_options": {"method": "statevector", "seed_simulator": 42},
        }
    )
    job = estimator.run([(isa_circuit, observable)])
    energy = float(job.result()[0].data.evs)
    return {"energy": energy, "backend": backend, "hamiltonian": hamiltonian, "num_qubits": circuit.num_qubits}


def minimize_vqe(
    path: str | None = None,
    initial_params: list[float] | None = None,
    iterations: int = 50,
    method: str = "pennylane",
) -> dict[str, Any]:
    if method == "pennylane":
        return minimize_vqe_pennylane(initial_params=initial_params, iterations=iterations)
    return minimize_vqe_scipy(initial_params=initial_params, iterations=iterations, path=path)


def minimize_vqe_pennylane(
    initial_params: list[float] | None = None,
    iterations: int = 80,
    stepsize: float = 0.4,
) -> dict[str, Any]:
    qml = _require_pennylane()
    from pennylane import numpy as pnp

    ctx = _get_h2_context()
    pl_h = ctx["H_pl"]
    n_qubits = ctx["n_qubits"]
    dev = qml.device("default.qubit", wires=n_qubits)

    @qml.qnode(dev, diff_method="adjoint")
    def cost(theta):
        qml.BasisState(ctx["hf"], wires=range(n_qubits))
        for idx, wires in enumerate(ctx["singles"]):
            qml.SingleExcitation(theta[idx], wires=wires)
        for idx, wires in enumerate(ctx["doubles"]):
            qml.DoubleExcitation(theta[len(ctx["singles"]) + idx], wires=wires)
        return qml.expval(pl_h)

    n_params = len(ctx["singles"]) + len(ctx["doubles"])
    if initial_params is not None:
        theta = pnp.array(initial_params, dtype=float, requires_grad=True)
        if len(theta) < n_params:
            theta = pnp.array(np.resize(np.asarray(initial_params, float), n_params), requires_grad=True)
    else:
        theta = pnp.zeros(n_params, requires_grad=True)

    opt = qml.GradientDescentOptimizer(stepsize=stepsize)
    energies: list[float] = []
    for _ in range(iterations):
        theta, e = opt.step_and_cost(cost, theta)
        energies.append(float(e))

    ref = ctx["ref"]
    return {
        "final_energy": float(energies[-1]),
        "energies": energies,
        "final_params": np.asarray(theta).tolist(),
        "reference_energy": ref,
        "error_hartree": abs(energies[-1] - ref),
        "method": "pennylane_ucc_adjoint",
    }


def minimize_vqe_scipy(
    initial_params: list[float] | None = None,
    iterations: int = 50,
    path: str | None = None,
) -> dict[str, Any]:
    from scipy.optimize import minimize

    p0 = np.zeros(3)
    if initial_params is not None:
        p0 = np.asarray(initial_params, dtype=float).flatten()
        if len(p0) < 3:
            p0 = np.resize(p0, 3)
    energies: list[float] = []

    def objective(p):
        e = run_qiskit(params=p.tolist())["energy"]
        energies.append(e)
        return e

    res = minimize(objective, p0, method="COBYLA", options={"maxiter": iterations, "rhobeg": 0.5})
    ref = h2_reference_energy()
    return {
        "final_energy": float(res.fun),
        "energies": energies,
        "final_params": res.x.tolist(),
        "reference_energy": ref,
        "error_hartree": abs(float(res.fun) - ref),
        "method": "qiskit_aer_scipy",
        "source": path,
    }


def circuit_from_physlang(path: str) -> dict[str, Any]:
    from physlang._native import run as native_run

    out = native_run(path)
    result: dict[str, Any] = dict(out)
    if out.get("circuit_json"):
        result["qiskit_circuit"] = circuit_ir_to_qiskit(out["circuit_json"])
    return result


def validate_h2_vqe(tolerance_mha: float = 1.0) -> dict[str, Any]:
    """Validate VQE against exact diagonalization (default 1 mHa)."""
    ctx = _get_h2_context()
    ref = ctx["ref"]
    pl = minimize_vqe_pennylane(iterations=80, stepsize=0.4)
    aer = run_qiskit(params=pl["final_params"])
    error_pl_mha = abs(pl["final_energy"] - ref) * 1000
    error_aer_mha = abs(aer["energy"] - ref) * 1000
    return {
        "reference_energy_hartree": ref,
        "pennylane_energy": pl["final_energy"],
        "qiskit_aer_energy": aer["energy"],
        "pennylane_error_mha": error_pl_mha,
        "qiskit_aer_error_mha": error_aer_mha,
        "passed": error_pl_mha <= tolerance_mha and error_aer_mha <= tolerance_mha,
        "tolerance_mha": tolerance_mha,
        "params": pl["final_params"],
    }
