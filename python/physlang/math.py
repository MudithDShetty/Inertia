"""PhysicsMath — dense LA, FFT, and numerical calculus via Rust core."""

from __future__ import annotations

from typing import Any

import numpy as np

from physlang._native import (
    math_bench_matmul,
    math_fft,
    math_matmul,
    math_solve,
    math_trapezoid,
)


def matmul(a: np.ndarray, b: np.ndarray) -> np.ndarray:
    """Matrix multiply using native ndarray/faer backend."""
    a = np.asarray(a, dtype=float)
    b = np.asarray(b, dtype=float)
    out = math_matmul(a.ravel().tolist(), list(a.shape), b.ravel().tolist(), list(b.shape))
    return np.array(out["data"], dtype=float).reshape(out["shape"])


def solve(a: np.ndarray, b: np.ndarray) -> np.ndarray:
    """Solve Ax = b for square A."""
    a = np.asarray(a, dtype=float)
    b = np.asarray(b, dtype=float).reshape(-1)
    n = a.shape[0]
    x = math_solve(a.ravel().tolist(), n, b.tolist())
    return np.array(x, dtype=float)


def bench_matmul(n: int = 1000, iters: int = 3) -> float:
    """Return mean milliseconds for n×n matmul."""
    return float(math_bench_matmul(n, iters))


def bench_matmul_vs_numpy(n: int = 1000, iters: int = 3) -> dict[str, Any]:
    """Compare native Rust matmul timing against NumPy (OpenBLAS on most installs)."""
    rng = np.random.default_rng(42)
    a = rng.standard_normal((n, n))
    b = rng.standard_normal((n, n))

    import time

    native_ms = bench_matmul(n, iters)

    t1 = time.perf_counter()
    for _ in range(iters):
        _ = a @ b
    numpy_ms = (time.perf_counter() - t1) * 1000.0 / iters

    ratio = native_ms / numpy_ms if numpy_ms > 0 else float("inf")
    return {
        "n": n,
        "native_ms": native_ms,
        "numpy_ms": numpy_ms,
        "ratio_native_over_numpy": ratio,
        "within_2x_openblas": ratio <= 2.0,
    }


def fft(x: np.ndarray) -> np.ndarray:
    """1-D FFT via rustfft."""
    spec = math_fft(np.asarray(x, dtype=float).ravel().tolist())
    return np.array(spec["re"]) + 1j * np.array(spec["im"])


def trapezoid(y: np.ndarray, dx: float) -> float:
    return float(math_trapezoid(np.asarray(y, dtype=float).ravel().tolist(), dx))
