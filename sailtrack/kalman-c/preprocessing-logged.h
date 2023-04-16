#ifndef PREPROCESSING_LOGGED_H
#define PREPROCESSING_LOGGED_H

#include <iostream>

int preprocess_logged_data(std::string);

struct IMU{
    double euler_x;
    double euler_y;
    double euler_z;
    double linearAccel_x;
    double linearAccel_y;
    double linearAccel_z;
};

struct ORIENTATION{
    double heading;
    double pitch;
    double roll;
};

struct GPS{
    double vAcc;
    double hAcc;
    double sAcc;
    double headAcc;
    double lon;
    double lat;
    double hMSL;
    double velN;
    double velE;
    double velD;
    double gSpeed;
    double headMot;
    double FL;
    double FR;
    double RL;
    double fixType;
    double epoch;
};

#endif  //PREPROCESSING_LOGGED_H
