#ifndef MODEL_H
#define MODEL_H 

#include "matrix-types.h"

namespace kf
{
    // Preliminary values

    /* IMPORTANT: SAMPLE TIME HAS TO MATCH THE REAL SAMPLE TIME OF THE KALMAN FILTER */
    //constexpr double kalman_sample_time_s{15};        // Sample time for the logged test data 
    constexpr double kalman_sample_time_s{0.2};         // Sample time for live data
    constexpr double w_std{0.1};                        // State noise (just a guess atm)
    constexpr double r_std{0.05};                       // not used, just for testing

    // Define (fixed) Model
    const fMatrix<6, 6> fF 
    {   
        {1, 0, 0, kalman_sample_time_s, 0, 0},
        {0, 1, 0, 0, kalman_sample_time_s, 0},
        {0, 0, 1, 0, 0, kalman_sample_time_s},
        {0, 0, 0, 1, 0, 0},
        {0, 0, 0, 0, 1, 0},
        {0, 0, 0, 0, 0, 1}
    };
    

    const fMatrix<6, 3> fG
    {
        {(kalman_sample_time_s*kalman_sample_time_s)/2, 0, 0},
        {0, (kalman_sample_time_s*kalman_sample_time_s)/2, 0},
        {0, 0, (kalman_sample_time_s*kalman_sample_time_s)/2},
        {kalman_sample_time_s, 0, 0},
        {0, kalman_sample_time_s, 0},
        {0, 0, kalman_sample_time_s}
    };
    
    const fMatrix<6, 6> fH{ fMatrix<6,6>::Identity() };

    const fMatrix<6, 6> fQ{ fG * fG.transpose() * (w_std*w_std) };
    
    // For test purpuses
    // fMatrix<6, 6> fR_default{(r_std*r_std) * fMatrix<6,6>::Identity()};
    const fMatrix<6, 6> fR_default 
    {   
        {1, 0, 0, 0, 0, 0},
        {0, 1, 0, 0, 0, 0},
        {0, 0, 1, 0, 0, 0},
        {0, 0, 0, 0.1, 0, 0},
        {0, 0, 0, 0, 0.1, 0},
        {0, 0, 0, 0, 0, 0.1}
    };

    // Values to initialize
    const kf::fMatrix<6,6> fP_init{kf::fQ};
    const kf::fVector<6> fx_init{kf::fVector<6>::Zero()};
} // namespace kf

#endif // MODEL_H