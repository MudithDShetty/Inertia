#!/usr/bin/env python3
"""Benchmark: PhysicsLang vs raw Qiskit for H2 VQE (line count + energy reference)."""

H2_VQE_PHYS = open("examples/quantum/h2_vqe.phys").read()
PHYS_LINES = len([l for l in H2_VQE_PHYS.splitlines() if l.strip() and not l.strip().startswith("//")])

QISKIT_EQUIVALENT = '''
from qiskit import QuantumCircuit
from qiskit.quantum_info import SparsePauliOp
from qiskit.primitives import Estimator
import numpy as np

def build_ansatz(theta):
    qc = QuantumCircuit(2)
    qc.ry(theta[0], 0)
    qc.ry(theta[1], 1)
    qc.cx(0, 1)
    qc.ry(theta[2], 0)
    return qc

H = SparsePauliOp.from_list([
    ("XX", -0.39), ("IZ", 0.18), ("ZI", 0.18),
])
estimator = Estimator()
theta = np.array([0.1, 0.1, 0.1, 0.1])
qc = build_ansatz(theta)
energy = estimator.run(qc, H).result().values[0]
print(f"Energy: {energy}")
'''

QISKIT_LINES = len([l for l in QISKIT_EQUIVALENT.splitlines() if l.strip() and not l.strip().startswith("#")])

print("=== PhysicsLang H2 VQE Benchmark ===")
print(f"PhysicsLang (.phys): ~{PHYS_LINES} lines")
print(f"Qiskit (Python):     ~{QISKIT_LINES} lines")
print(f"Reduction:           ~{100 - (PHYS_LINES / QISKIT_LINES * 100):.0f}% fewer lines")
print()
print("Reference H2 ground-state energy (STO-3G): ~-1.137 Ha")
print("Run: phys run examples/quantum/h2_vqe.phys")

try:
    import physlang
    result = physlang.run("examples/quantum/h2_vqe.phys")
    print(f"PhysicsLang result: {result['result']}")
except ImportError:
    print("(Install Python bindings: maturin develop)")
