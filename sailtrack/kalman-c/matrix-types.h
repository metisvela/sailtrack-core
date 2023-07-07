#ifndef MATRIX_TYPES_H
#define MATRIX_TYPES_H

// TODO - CHANGE EIGEN LIBRARY PATH TO YOUR OWN
#include </usr/include/eigen3/Eigen/Dense>

// Useful Aliases
namespace kf
{
    // Dynamic types
    using dMatrix = Eigen::Matrix<double, Eigen::Dynamic, Eigen::Dynamic>;
    using dVector = Eigen::Matrix<double, Eigen::Dynamic, 1>;

    // Fixed Types
    //TODO - Check float vs double on real plattform
    template <size_t ROW, size_t COL>
    using fMatrix = Eigen::Matrix<double, ROW, COL>;

    template <size_t ROW>
    using fVector = Eigen::Matrix<double, ROW, 1>;
}

#endif //MATRIX_TYPES_H