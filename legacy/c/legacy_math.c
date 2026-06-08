/* Legacy C math routines — Fortran-compatible ABI (BIND(C) style).
 * A Fortran equivalent lives in legacy/fortran/daxpy.f90.
 */

#ifdef _WIN32
#define PHYS_EXPORT __declspec(dllexport)
#else
#define PHYS_EXPORT
#endif

PHYS_EXPORT void phys_daxpy(int n, double alpha, const double *x, double *y) {
    for (int i = 0; i < n; ++i) {
        y[i] += alpha * x[i];
    }
}

PHYS_EXPORT double phys_dot(int n, const double *x, const double *y) {
    double sum = 0.0;
    for (int i = 0; i < n; ++i) {
        sum += x[i] * y[i];
    }
    return sum;
}

PHYS_EXPORT double phys_fortran_pi(void) {
    return 3.14159265358979323846;
}
