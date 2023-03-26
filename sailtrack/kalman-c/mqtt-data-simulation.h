#ifndef MQTT_DATA_SIMULATION_H
#define MQTT_DATA_SIMULATION_H

#include <iostream>

int preprocess_simulation_data(std::string);

struct IMU{
    double euler_x;
    double euler_y;
    double euler_z;
    double linearAccel_x;
    double linearAccel_y;
    double linearAccel_z;
}imu_data;

struct ORIENTATION{
    double heading;
    double pitch;
    double roll;
}orientation;

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
}gps_data;

#endif  //MQTT_DATA_SIMULATION_H
