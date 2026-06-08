"""Legacy C/Fortran-compatible FFI bridge."""

from __future__ import annotations

from physlang._native import interop_daxpy, interop_dot, interop_legacy_pi


def daxpy(alpha: float, x: list[float], y: list[float]) -> list[float]:
    """BLAS-style y = alpha * x + y via legacy C library."""
    return interop_daxpy(alpha, x, y)


def dot(x: list[float], y: list[float]) -> float:
    return float(interop_dot(x, y))


def legacy_pi() -> float:
    return float(interop_legacy_pi())


def validate_extern_daxpy() -> dict[str, object]:
    """Phase 2 exit check: extern DAXPY matches BLAS semantics."""
    x = [1.0, 2.0, 3.0]
    y = [4.0, 5.0, 6.0]
    out = daxpy(2.0, x, y)
    expected = [6.0, 9.0, 12.0]
    ok = all(abs(a - b) < 1e-12 for a, b in zip(out, expected))
    return {"x": x, "y_in": y, "y_out": out, "expected": expected, "passed": ok}
