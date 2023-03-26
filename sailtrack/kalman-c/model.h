#ifndef MODEL_H
#define MODEL_H 

#include "matrix-types.h"

namespace kf
{
    constexpr float sample_time{1};
    constexpr float w_std{0.1};
    constexpr float r_std{0.05};

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
} // namespace kf

#endif // MODEL_H