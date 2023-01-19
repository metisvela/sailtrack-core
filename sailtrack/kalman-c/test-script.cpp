#include <iostream>
#include <chrono>
#include <ctime>
#include "fixed-kalman-filter.h"
#include "dynamic-kalman-filter.h"


using namespace kf;

static constexpr float sample_time{0.1};
static constexpr float w_std{0.1};


int main() {

    // Define (fixed) Model
    fMatrix<6, 6> fF;
    fF << 1, 0, 0, sample_time, 0, 0, 
         0, 1, 0, 0, sample_time, 0,
         0, 0, 1, 0, 0, sample_time,
         0, 0, 0, 1, 0, 0,
         0, 0, 0, 0, 1, 0,
         0, 0, 0, 0, 0, 1;
    
    fMatrix<6, 3> fG;
    fG << (sample_time*sample_time)/2, 0, 0,
         0, (sample_time*sample_time)/2, 0,
         0, 0, (sample_time*sample_time)/2,
         sample_time, 0, 0,
         0, sample_time, 0,
         0, 0, sample_time;
    
    fMatrix<6, 6> fH{ fMatrix<6,6>::Identity() };

    fMatrix<6, 6> fQ;
    fQ = fG * fG.transpose() * (w_std*w_std);

    fMatrix<6,6> fP_init;
    fP_init = fQ;

    fVector<6> fx_init{fVector<6>::Zero()};
    
    fMatrix<6, 6> fR{0.1 * fMatrix<6,6>::Identity()};

    // Define (dynamic) Model
    dMatrix dF = fF.eval();
    dMatrix dG = fG.eval();
    dMatrix dH = fH.eval();
    dMatrix dQ = fQ.eval();
    dMatrix dR = fR.eval();
    dMatrix dP_init = fP_init.eval();
    dVector dx_init = fx_init.eval();


    // Initialize Filters
    FixedKalmanFilter<6, 3, 6> fFilter(fF, fG, fH, fQ, fx_init, fP_init);
    // DynamicKalmanFilter dFilter(dF, dG, dH, dQ, dx_init, dP_init);

    // Create dummy input and measurment vectors
    fVector<3> finput_vector;
    // dVector dinput_vector;
    finput_vector << 1, 1, 1;
    // dinput_vector = finput_vector.eval();

    fVector<6> fmeasurement_vector;
    dVector dmeasurement_vector;
    fmeasurement_vector << 1, 1, 1, 1, 1, 1;
    dmeasurement_vector = fmeasurement_vector.eval();

    // Run and time test computation
    std::chrono::time_point<std::chrono::high_resolution_clock> start, end;

    start = std::chrono::high_resolution_clock::now();
    
    /* On my machine, using -O2 -march=native -DNDEBUG to compile ,
       the following code with the fixed size kalman filter was ~100 times
       faster than the equivalent python code. 
       More than 50 times faster when loop limit is set to 1.*/
    for (size_t i = 0; i < 10000; i++) {
        fFilter.update_noise_covariance(fR);
        fFilter.predict(finput_vector);
        fFilter.correct(fmeasurement_vector);

        //dFilter.update_noise_covariance(fR);
        //dFilter.predict(finput_vector);
        //dFilter.correct(fmeasurement_vector);
    }
    end = std::chrono::high_resolution_clock::now();
  
    // Print the current state estimate and passed time
    std::cout << fFilter.state_estimate() << std::endl;
    std::cout << fFilter.state_covariance_estimate() << std::endl;
    //std::cout << fFilter.state_estimate() << std::endl;
    //std::cout << fFilter.state_covariance_estimate() << std::endl;


    std::cout << std::chrono::duration_cast<std::chrono::microseconds>(end-start).count() << std::endl;

    return 0;
}
