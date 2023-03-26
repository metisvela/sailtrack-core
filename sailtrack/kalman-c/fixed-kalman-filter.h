/* TODO
 *        
 *       - Do some Testing 
 * 
 *	     - Check numerical stability of implementation (P staying p.s.d. for prolonged amount of time)
 *	      -- if unstable, change algorithm to more stable one (e.g. square root formulation)
 *
 *       Possible optimization routes:
 *         - Use model specific simplifications (R being diagonal)
 *         - specify model at compile-time, i.e. fully templetize or constexpr everything  
 *         - study the Eigen library documentation at https://eigen.tuxfamily.org/dox/
 *             -- e. g. passing matrices as arguments: https://eigen.tuxfamily.org/dox/TopicFunctionTakingEigenTypes.html
 *                 --- pass arguments with Eigen::Ref?? (no temporaries but losing alignment information)
 */

#ifndef FIXED_KALMAN_FILTER_H
#define FIXED_KALMAN_FILTER_H

// YOU NEED TO ADD YOUR EIGEN LIBRARY PATH YOURSELF
#include <Eigen/Dense> // TODO - Make this library work in PlatformIO
#include "matrix-types.h"

namespace kf
{
  /** Generic implementation of a Kalman Filter using Eigen library.
   *  Matrices have fixed size, determined by template parameters.
   * 
   *  Preliminary test shows that for the metis vela 6D model the fixed
   *  sized Kalman filter is apprx. twice as fast as the dynamically
   *  sized one.
   *
   *  N - state dim, M - input dim, P - output dim
   */
  template <size_t DIM_N, size_t DIM_M, size_t DIM_P> // N - state dim, M - input dim, P - output dim
  class FixedKalmanFilter
  {
  public:
    FixedKalmanFilter(const fMatrix<DIM_N, DIM_N> &F, const fMatrix<DIM_N, DIM_M> &G, const fMatrix<DIM_P, DIM_N> &H,
                      const fMatrix<DIM_N, DIM_N> &Q);
    FixedKalmanFilter(const fMatrix<DIM_N, DIM_N> &F, const fMatrix<DIM_N, DIM_M> &G, const fMatrix<DIM_P, DIM_N> &H,
                      const fMatrix<DIM_N, DIM_N> &Q, const fVector<DIM_N> &x_hat_init, const fMatrix<DIM_N, DIM_N> &P_init);

    // Time Update
    void predict();
    void predict(const fVector<DIM_M> &input_vector);
    void predict(const fVector<DIM_M> &input_vector, const fVector<DIM_P> &measurement_vector);

    // Measurement Update
    void correct(const fVector<DIM_P> &measurement_vector);

    // Member access
    const fVector<DIM_N> &state_estimate() const { return x_hat_; }
    const fMatrix<DIM_N, DIM_N> &state_covariance_estimate() const { return P_; }

    // Member update
    void update_noise_covariance(const fMatrix<DIM_P, DIM_P> &R_new) { R_ = R_new; }

  private:
    // TODO - Change to const where appropriate, eventually constexpr for fixed model if necessary
    const fMatrix<DIM_N, DIM_N> F_; // state transition
    const fMatrix<DIM_N, DIM_M> G_; // input/control
    const fMatrix<DIM_P, DIM_N> H_; // output/observation
    const fMatrix<DIM_N, DIM_N> Q_; // state noise covariance
    fMatrix<DIM_P, DIM_P> R_;       // measurment noise covariance
    fMatrix<DIM_N, DIM_P> S_;       // measure-to-state uncorrelation (?)
    fVector<DIM_N> x_hat_;          // state estimate
    fMatrix<DIM_N, DIM_N> P_;       // state covariance estimate
  };

  template <size_t DIM_N, size_t DIM_M, size_t DIM_P>
  FixedKalmanFilter<DIM_N, DIM_M, DIM_P>::FixedKalmanFilter(const fMatrix<DIM_N, DIM_N> &F, const fMatrix<DIM_N, DIM_M> &G, const fMatrix<DIM_P, DIM_N> &H,
                                                            const fMatrix<DIM_N, DIM_N> &Q)
      : F_{F},
        G_{G},
        H_{H},
        Q_{Q},
        R_{fMatrix<DIM_P, DIM_P>::Zero()},
        S_{fMatrix<DIM_N, DIM_P>::Zero()},
        x_hat_{fVector<DIM_N>::Zero()},
        P_{fMatrix<DIM_N, DIM_N>::Identity()}
  {
  }

  template <size_t DIM_N, size_t DIM_M, size_t DIM_P>
  FixedKalmanFilter<DIM_N, DIM_M, DIM_P>::FixedKalmanFilter(const fMatrix<DIM_N, DIM_N> &F, const fMatrix<DIM_N, DIM_M> &G, const fMatrix<DIM_P, DIM_N> &H,
                                                            const fMatrix<DIM_N, DIM_N> &Q, const fVector<DIM_N> &x_hat_init, const fMatrix<DIM_N, DIM_N> &P_init)
      : F_{F},
        G_{G},
        H_{H},
        Q_{Q},
        R_{fMatrix<DIM_P, DIM_P>::Zero()},
        S_{fMatrix<DIM_N, DIM_P>::Zero()},
        x_hat_{x_hat_init},
        P_{P_init}
  {
  }

  template <size_t DIM_N, size_t DIM_M, size_t DIM_P>
  void FixedKalmanFilter<DIM_N, DIM_M, DIM_P>::predict()
  {
    x_hat_ = F_ * x_hat_;               // extrapolate state
    P_ = F_ * P_ * F_.transpose() + Q_; // extrapolate state covariance
  }

  template <size_t DIM_N, size_t DIM_M, size_t DIM_P>
  void FixedKalmanFilter<DIM_N, DIM_M, DIM_P>::predict(const fVector<DIM_M> &input_vector)
  {
    x_hat_ = F_ * x_hat_ + G_ * input_vector; // extrapolate state
    P_ = F_ * P_ * F_.transpose() + Q_;       // extrapolate state covariance
  }

  template <size_t DIM_N, size_t DIM_M, size_t DIM_P>
  void FixedKalmanFilter<DIM_N, DIM_M, DIM_P>::predict(const fVector<DIM_M> &input_vector, const fVector<DIM_P> &measurement_vector)
  {
    // NOTE - R is diagonal in our model, can be optimized.
    x_hat_ = F_ * x_hat_ + G_ * input_vector + S_ * R_.ldlt().solve(measurement_vector); // extrapolate state
    P_ = F_ * P_ * F_.transpose() + Q_;                                                  // extrapolate state covariance
  }

  template <size_t DIM_N, size_t DIM_M, size_t DIM_P>
  void FixedKalmanFilter<DIM_N, DIM_M, DIM_P>::correct(const fVector<DIM_P> &measurement_vector)
  {
    // Original formulation: K = P*H.T*(H*P*H.T + R)^-1
    // Equivalent to: K.T = linalg.solve((HPH.T+R).T, HP.T)

    // Compute Kalman Gain using p.s.d. LDLT solver of Eigen library
    const fMatrix<DIM_N, DIM_P> K{((H_ * P_ * H_.transpose() + R_).transpose()).ldlt().solve(H_ * P_.transpose()).transpose()};

    // Update the state estimate
    x_hat_ += K * (measurement_vector - H_ * x_hat_); // Update state estimate using measurement
    P_ -= K * H_ * P_;                                // Update covariance estimate using measurement  //TODO - Check wether P stays p.s.d. over many function calls
  }

}

#endif // FIXED_KALMAN_FILTER_H
