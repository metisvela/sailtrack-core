#ifndef MODEL_H
#define MODEL_H 

#include "matrix-types.h"

// TODO Fix for the real model!
namespace kf
{
    // Preliminary values
    constexpr float sample_time{15}; // IMPORTANT: HAS TO MATCH THE REAL SAMPLE TIME OF THE DATA
    constexpr float w_std{0.1};  // State noise is just a guess atm
    constexpr float r_std{0.05}; // not used, just for testing

    // Define (fixed) Model
    const fMatrix<6, 6> fF 
    {   
        {1, 0, 0, sample_time, 0, 0},
        {0, 1, 0, 0, sample_time, 0},
        {0, 0, 1, 0, 0, sample_time},
        {0, 0, 0, 1, 0, 0},
        {0, 0, 0, 0, 1, 0},
        {0, 0, 0, 0, 0, 1}
    };
    

    const fMatrix<6, 3> fG
    {
        {(sample_time*sample_time)/2, 0, 0},
        {0, (sample_time*sample_time)/2, 0},
        {0, 0, (sample_time*sample_time)/2},
        {sample_time, 0, 0},
        {0, sample_time, 0},
        {0, 0, sample_time}
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
} // namespace kf

#endif // MODEL_H