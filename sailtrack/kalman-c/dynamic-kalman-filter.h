/* TODO - Do some testing
 */

#ifndef DYNAMIC_KALMAN_FILTER_H
#define DYNAMIC_KALMAN_FILTER_H

// YOU NEED TO ADD YOUR EIGEN LIBRARY PATH YOURSELF
#include <Eigen/Dense> // TODO - Make this library work in PlatformIO
#include "matrix-types.h"

namespace kf
{

    /** Generic implementation of a Kalman Filter using Eigen library.
     *  Matrices have dynamic size.
     */
    class DynamicKalmanFilter
    {
    public:
        DynamicKalmanFilter(const dMatrix &F, const dMatrix &G, const dMatrix &H,
                            const dMatrix &Q);
        DynamicKalmanFilter(const dMatrix &F, const dMatrix &G, const dMatrix &H,
                            const dMatrix &Q, const dVector &x_hat_init, const dMatrix &P_init);

        // Time Update
        void predict();
        void predict(const dVector &input_vector);
        void predict(const dVector &input_vector, const dVector &measurement_vector);

        // Measurement Update
        void correct(const dVector &measurement_vector);

        // Member access
        const dVector &state_estimate() const { return x_hat_; }
        const dMatrix &state_covariance_estimate() const { return P_; }

        // Member update
        void update_noise_covariance(const dMatrix &R_new) { R_ = R_new; }

    private:
        // TODO - Change to const where appropriate, eventually constexpr for fixed model if necessary
        const dMatrix F_; // state transition
        const dMatrix G_; // input/control
        const dMatrix H_; // output/observation
        const dMatrix Q_; // state noise covariance
        dMatrix R_;       // measurment noise covariance
        dMatrix S_;       // measure-to-state uncorrelation (?)
        dVector x_hat_;   // state estimate
        dMatrix P_;       // state covariance estimate
    };

    DynamicKalmanFilter::DynamicKalmanFilter(const dMatrix &F, const dMatrix &G, const dMatrix &H,
                                             const dMatrix &Q)
        : F_{F},
          G_{G},
          H_{H},
          Q_{Q},
          R_{dMatrix::Zero(H.rows(), H.cols())},    // NOTE: THIS ONLY WORKS IF DECLARATION OF H_ COMES BEFORE R_
          S_{dMatrix::Zero(F.rows(), H.cols())},    // NOTE: THIS ONLY WORKS IF DECLARATION OF F_, H_ COMES BEFORE S_
          x_hat_{dVector::Zero(F.rows())},          // NOTE: THIS ONLY WORKS IF DECLARATION OF F_ COMES BEFORE X_HAT_
          P_{dMatrix::Identity(F.rows(), F.cols())} // NOTE: THIS ONLY WORKS IF DECLARATION OF F_ COMES BEFORE P_
    {
    }

    DynamicKalmanFilter::DynamicKalmanFilter(const dMatrix &F, const dMatrix &G, const dMatrix &H,
                                             const dMatrix &Q, const dVector &x_hat_init, const dMatrix &P_init)
        : F_{F},
          G_{G},
          H_{H},
          Q_{Q},
          R_{dMatrix::Zero(H.rows(), H.cols())}, // NOTE: THIS ONLY WORKS IF DECLARATION OF H_ COMES BEFORE R_
          S_{dMatrix::Zero(F.rows(), F.cols())}, // NOTE: THIS ONLY WORKS IF DECLARATION OF A_, H_ COMES BEFORE S_
          x_hat_{x_hat_init},
          P_{P_init}
    {
    }

    void DynamicKalmanFilter::predict()
    {
        x_hat_ = F_ * x_hat_;               // extrapolate state
        P_ = F_ * P_ * F_.transpose() + Q_; // extrapolate state covariance
    }

    void DynamicKalmanFilter::predict(const dVector &input_vector)
    {
        x_hat_ = F_ * x_hat_ + G_ * input_vector; // extrapolate state
        P_ = F_ * P_ * F_.transpose() + Q_;       // extrapolate state covariance
    }

    void DynamicKalmanFilter::predict(const dVector &input_vector, const dVector &measurement_vector)
    {
        // NOTE - R is diagonal in our model, can be optimized.
        x_hat_ = F_ * x_hat_ + G_ * input_vector + S_ * R_.ldlt().solve(measurement_vector); // extrapolate state
        P_ = F_ * P_ * F_.transpose() + Q_;                                                  // extrapolate state covariance
    }

    void DynamicKalmanFilter::correct(const dVector &measurement_vector)
    {
        // Original formulation: K = P*H.T*(H*P*H.T + R)^-1
        // Equivalent to: K.T = linalg.solve((HPH.T+R).T, HP.T)

        // Compute Kalman Gain using p.s.d. LDLT solver of Eigen library
        const dMatrix K{((H_ * P_ * H_.transpose() + R_).transpose()).ldlt().solve(H_ * P_.transpose()).transpose()};

        // Update the state estimate
        x_hat_ += K * (measurement_vector - H_ * x_hat_); // Update state estimate using measurement
        P_ -= K * H_ * P_;                                // Update covariance estimate using measurement
    }

}

#endif // DYNAMIC_KALMAN_FILTER_H
