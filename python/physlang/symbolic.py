"""SymPy integration via Python FFI for symbolic calculus."""

from __future__ import annotations

from typing import Any


def _require_sympy():
    try:
        import sympy as sp
    except ImportError as e:
        raise ImportError("Install symbolic extras: pip install physlang[symbolic]") from e
    return sp


def differentiate(expr: str, variable: str = "x") -> str:
    """Differentiate a symbolic expression string."""
    sp = _require_sympy()
    x = sp.Symbol(variable)
    e = sp.sympify(expr)
    return str(sp.diff(e, x))


def simplify(expr: str) -> str:
    sp = _require_sympy()
    return str(sp.simplify(sp.sympify(expr)))


def substitute(expr: str, variable: str, value: float) -> float:
    sp = _require_sympy()
    x = sp.Symbol(variable)
    return float(sp.sympify(expr).subs(x, value))


def symbolic_to_latex(expr: str) -> str:
    sp = _require_sympy()
    return sp.latex(sp.sympify(expr))


def validate_symbolic_diff() -> dict[str, Any]:
    """Phase 2 exit check: d/dx x^2 = 2x."""
    result = differentiate("x**2", "x")
    simplified = simplify(result)
    ok = simplified in ("2*x", "2x")
    return {
        "input": "x**2",
        "derivative": result,
        "simplified": simplified,
        "passed": ok,
    }
