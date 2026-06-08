#!/usr/bin/env python3
"""Run H2 VQE validation — requires: pip install -e '.[quantum]'"""

from physlang.quantum import validate_h2_vqe, minimize_vqe, run_qiskit, h2_reference_energy

if __name__ == "__main__":
    print("H2 reference (exact):", h2_reference_energy(), "Ha")
    result = validate_h2_vqe()
    print("PennyLane VQE:", result["pennylane_energy"], "Ha")
    print("Qiskit Aer:", result["qiskit_aer_energy"], "Ha")
    print("PennyLane error:", result["pennylane_error_mha"], "mHa")
    print("Aer error:", result["qiskit_aer_error_mha"], "mHa")
    print("PASSED" if result["passed"] else "FAILED")
    raise SystemExit(0 if result["passed"] else 1)
