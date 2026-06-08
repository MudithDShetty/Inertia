#!/usr/bin/env python3
"""Phase 2 validation — LA benchmark, SymPy diff, legacy DAXPY."""

from physlang.interop import validate_extern_daxpy
from physlang.math import bench_matmul, bench_matmul_vs_numpy, matmul, solve
from physlang.symbolic import validate_symbolic_diff

import numpy as np


def main() -> int:
    print("=== Phase 2 validation ===")

    # Dense LA
    a = np.array([[4.0, 1.0], [1.0, 3.0]])
    b = np.array([[1.0], [2.0]])
    x = solve(a, b)
    print("solve 2x2:", x.ravel())
    c = matmul(a, np.eye(2))
    print("matmul identity:", np.allclose(c, a))

    bench = bench_matmul_vs_numpy(n=1000, iters=3)
    native_ms = bench_matmul(n=1000, iters=3)
    print(
        f"matmul 1000x1000: native={native_ms:.2f}ms "
        f"numpy={bench['numpy_ms']:.2f}ms ratio={native_ms / bench['numpy_ms']:.2f}"
    )

    sym = validate_symbolic_diff()
    print("SymPy d/dx x^2:", sym["simplified"], "PASSED" if sym["passed"] else "FAILED")

    ext = validate_extern_daxpy()
    print("extern daxpy:", ext["y_out"], "PASSED" if ext["passed"] else "FAILED")

    ratio = native_ms / bench["numpy_ms"] if bench["numpy_ms"] > 0 else float("inf")
    la_ok = ratio <= 2.0
    passed = sym["passed"] and ext["passed"] and la_ok
    print("OVERALL:", "PASSED" if passed else "FAILED")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
