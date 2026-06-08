"""PhysicsLang — Python package (Rust core + quantum backends)."""

from __future__ import annotations

from typing import Any

from physlang._native import (
    circuit_svg,
    compile_file,
    compile_source,
    convergence_plot_script,
    numpy_params_buffer,
    run as _run_native,
    to_qiskit,
)
from physlang.quantum import (
    circuit_from_physlang,
    h2_reference_energy,
    minimize_vqe,
    run_qiskit,
    validate_h2_vqe,
)
from physlang.math import bench_matmul, bench_matmul_vs_numpy, matmul, solve, fft, trapezoid
from physlang.symbolic import differentiate, simplify, validate_symbolic_diff
from physlang.interop import daxpy, dot, validate_extern_daxpy

__all__ = [
    "compile_file",
    "compile_source",
    "run",
    "to_qiskit",
    "circuit_svg",
    "numpy_params_buffer",
    "convergence_plot_script",
    "run_qiskit",
    "minimize_vqe",
    "validate_h2_vqe",
    "h2_reference_energy",
    "circuit_from_physlang",
    "matmul",
    "solve",
    "bench_matmul",
    "bench_matmul_vs_numpy",
    "fft",
    "trapezoid",
    "differentiate",
    "simplify",
    "validate_symbolic_diff",
    "daxpy",
    "dot",
    "validate_extern_daxpy",
]


def run(path: str, entry: str | None = None) -> dict[str, Any]:
    """Compile and run a .phys file via the native interpreter."""
    return _run_native(path, entry)
