#include <iostream>
#include <chrono>
#include <ctime>
#include <random>

#include "fixed-kalman-filter.h"
#include "dynamic-kalman-filter.h"
#include "eigen-2-csv.h"
#include "model.h" // TODO: Fix model sample time and Q for the real model in model.h
#include "preprocessing-logged.h"

#define SENSOR_RAW_DATA "test_data/sensor-data-2022-09-22_no_wind.csv"


using namespace kf;

int main() {

    // Stuff to initialize the filter
    fMatrix<6,6> fP_init {fQ};
    fVector<6> fx_init{fVector<6>::Zero()};
    // fMatrix<6, 6> fR{(r_std*r_std) * fMatrix<6,6>::Identity()}; // if datageneration.m is used

    // Initialize Filter
    FixedKalmanFilter<6, 3, 6> fFilter(fF, fG, fH, fQ, fx_init, fP_init);

    // Preprocess data from raw data file. Creates seperate csv files for inputs, measurments and noise covariance
    preprocess_logged_data(SENSOR_RAW_DATA);
    
    //Load Data
    dMatrix input_vectors = (csv2eigen("test_data/acc_input.csv")).transpose();     // transpose to get column vector for each entry
    dMatrix measurement_vectors = (csv2eigen("test_data/measure.csv")).transpose(); // transpose to get column vector for each entry
    dMatrix R_diags = (csv2eigen("test_data/kalman_boat_R.csv")).transpose();       // load diagonal values into column vectors

    // Vector that will save state estimates
    std::vector<dVector> state_estimates{};

    // Run the filter
    for (size_t i = 0; i < input_vectors.cols(); i++) {

        // Prediction using model and acceleration measurment
        if(!input_vectors.col(i).hasNaN()) 
        { 
            fFilter.predict(input_vectors.col(i));
        } else {
            fFilter.predict(fVector<3>::Zero());  // No data, assume acceleration is zero. (Conserving Inertia.)
        }

        // Correct/Filter with state measurement if gps_data is given
        if( (!measurement_vectors.col(i).hasNaN()) && (!R_diags.col(i).hasNaN()) ) {
            fFilter.update_noise_covariance(R_diags.col(i).asDiagonal());
            // fFilter.update_noise_covariance(fR_default); // if KF is stable, P should converge for static R

            fFilter.correct(measurement_vectors.col(i));
        }

        state_estimates.push_back(fFilter.state_estimate());
    }

    std::cout << fFilter.error_covariance_estimate() << std::endl;

    // Save state estimates (row-wise)
    eigen2csv("test_data/estimates.csv", state_estimates);

    return 0;
}