! Fortran DAXPY with C binding — mirrors legacy/c/legacy_math.c
module phys_legacy
  implicit none
contains
  subroutine daxpy(n, alpha, x, y)
    use iso_c_binding
    integer(c_int), value :: n
    real(c_double), value :: alpha
    real(c_double), intent(in) :: x(n)
    real(c_double), intent(inout) :: y(n)
    integer :: i
    do i = 1, n
      y(i) = y(i) + alpha * x(i)
    end do
  end subroutine daxpy
end module phys_legacy
