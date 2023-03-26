#include <iostream>
#include <fstream>
#include <sstream>
#include <vector>
#include <string>
#include <cmath>
#include <stdexcept>
#include <limits>

#include "mqtt-data-simulation.h"

constexpr double EARTH_CIRCUMFERENCE_METERS{40075.0 * 1000.0};
constexpr double MPS_TO_KNOTS_MULTIPLIER{1.94384};

using namespace std;

bool gps_available = true;
bool imu_available = true;
GPS *gps_ref = nullptr;
double LAT_FACTOR = 1;

vector<double> acc_input;     //  used as input for the
vector<double> kalman_boat_R; //  print_data_csv function
vector<double> measure_v;     //

double nanValue = numeric_limits<double>::quiet_NaN();                                         //  default values printed in the
vector<double> no_acc_input = {nanValue, nanValue, nanValue};                                  //  csv when data not available
vector<double> no_R_no_measure = {nanValue, nanValue, nanValue, nanValue, nanValue, nanValue}; //

// helper functions
double toRadians(double);
void printdata(vector<vector<string>>);
int print_data_csv(string, vector<double>);

// Function takes raw sensor csv-file,
// creates a 3 csv-files with preprocessed acceleration, measurment and noise covariance data.
int preprocess_simulation_data(string filename)
{
    remove("../test_data/acc_input.csv");     // remove previous version of the output files
    remove("../test_data/kalman_boat_R.csv"); //
    remove("../test_data/measure.csv");       //

    vector<vector<string>> data;
    ifstream file(filename);
    string line;
    while (getline(file, line))
    {
        vector<string> row;
        stringstream ss(line);
        string value;
        while (getline(ss, value, ','))
        {
            row.push_back(value);
        }
        data.push_back(row);
    }
    file.close();

    for (int i = 1; i < data.size(); i++)
    {
        try
        {
            // "no wind" raw data
            imu_data.euler_x = stod(data[i][2]);
            imu_data.euler_y = stod(data[i][3]);
            imu_data.euler_z = stod(data[i][4]);
            imu_data.linearAccel_x = stod(data[i][12]);
            imu_data.linearAccel_y = stod(data[i][13]);
            imu_data.linearAccel_z = stod(data[i][14]);

            // other raw data
            // imu_data.euler_x = stod(data[i][1]);
            // imu_data.euler_y = stod(data[i][2]);
            // imu_data.euler_z = stod(data[i][3]);
            // imu_data.linearAccel_x = stod(data[i][4]);
            // imu_data.linearAccel_y = stod(data[i][5]);
            // imu_data.linearAccel_z = stod(data[i][6]);

            orientation.heading = 360 - imu_data.euler_z;
            orientation.pitch = -imu_data.euler_y;
            orientation.roll = imu_data.euler_x;

            imu_available = true;
        }
        catch (const invalid_argument &e)
        {
            imu_available = false; // at leat one of imu metrics is not available
        }
        try
        {
            //  "No wind" raw data
            gps_data.vAcc = stod(data[i][17]) * pow(10, -3);
            gps_data.hAcc = stod(data[i][7]) * pow(10, -3);
            gps_data.sAcc = stod(data[i][16]) * pow(10, -3);
            gps_data.headAcc = stod(data[i][9]);
            gps_data.lon = stod(data[i][15]) * pow(10, -7);
            gps_data.lat = stod(data[i][11]) * pow(10, -7);
            gps_data.hMSL = stod(data[i][8]) * pow(10, -3);
            gps_data.velN = stod(data[i][20]) * pow(10, -3);
            gps_data.velE = stod(data[i][19]) * pow(10, -3);
            gps_data.velD = stod(data[i][18]) * pow(10, -3);
            gps_data.gSpeed = stod(data[i][6]);
            gps_data.headMot = stod(data[i][10]);
            gps_data.fixType = stod(data[i][5]);
            gps_data.epoch = stod(data[i][1]);
            // gps_data.FL = stod(data[i][19]);
            // gps_data.FR = stod(data[i][20]);
            // gps_data.RL = stod(data[i][21]);

            // other raw data
            // gps_data.vAcc = stod(data[i][7]) * pow(10, -3);
            // gps_data.hAcc = stod(data[i][8]) * pow(10, -3);
            // gps_data.sAcc = stod(data[i][9]) * pow(10, -3);
            // gps_data.headAcc = stod(data[i][10]);
            // gps_data.lon = stod(data[i][11]) * pow(10, -7);
            // gps_data.lat = stod(data[i][12]) * pow(10, -7);
            // gps_data.hMSL = stod(data[i][13]) * pow(10, -3);
            // gps_data.velN = stod(data[i][14]) * pow(10, -3);
            // gps_data.velE = stod(data[i][15]) * pow(10, -3);
            // gps_data.velD = stod(data[i][16]) * pow(10, -3);
            // gps_data.gSpeed = stod(data[i][17]);
            // gps_data.headMot = stod(data[i][18]);
            // // gps_data.fixType = stod(data[i][5]);
            // // gps_data.epoch = stod(data[i][1]);
            // gps_data.FL = stod(data[i][19]);
            // gps_data.FR = stod(data[i][20]);
            // gps_data.RL = stod(data[i][21]);

            gps_available = true;

            if (gps_ref == nullptr)
                gps_ref = &gps_data;
        }
        catch (const invalid_argument &e)
        {
            gps_available = false; // at leat one of gps metrics is not available
        }

        if (imu_available)
        {

            double w2b_mtx[3][3] = {{cos(toRadians(orientation.heading)), sin((toRadians(orientation.heading))), 0},
                                    {-sin((toRadians(orientation.heading))), cos(toRadians(orientation.heading)), 0},
                                    {0, 0, 1}};

            double linear_accel_wrf[3] = {w2b_mtx[0][0] * imu_data.linearAccel_x + w2b_mtx[0][1] * imu_data.linearAccel_y,
                                          w2b_mtx[1][0] * imu_data.linearAccel_x + w2b_mtx[1][1] * imu_data.linearAccel_y,
                                          imu_data.linearAccel_z};

            for (int i = 0; i < 3; i++)
            {
                acc_input.push_back(linear_accel_wrf[i]); // acceleration input after processing
            }                                             // 1x3 vector
        }

        if (gps_available)
        {

            LAT_FACTOR = cos(toRadians(gps_data.lat + (*gps_ref).lat) / 2);

            double gps_rel_pos[3] = {(gps_data.lat - (*gps_ref).lat) * EARTH_CIRCUMFERENCE_METERS / 360,
                                     (gps_data.lon - (*gps_ref).lon) * EARTH_CIRCUMFERENCE_METERS * LAT_FACTOR / 360,
                                     (gps_data.hMSL - (*gps_ref).hMSL)};

            double gps_ned_velocity[3] = {gps_data.velN, gps_data.velE, -gps_data.velD};

            double kalman_boar_R[6] = {pow(gps_data.hAcc, 2) * 0.25, pow(gps_data.hAcc, 2) * 0.25, pow(gps_data.vAcc, 2) * 0.25,
                                       pow(gps_data.sAcc, 2) * 0.25, pow(gps_data.sAcc, 2) * 0.25, pow(gps_data.sAcc, 2) * 0.25};

            for (int i = 0; i < 6; i++)
            {
                kalman_boat_R.push_back(kalman_boar_R[i]); // R matrix after processing. These six elements are the element
            }                                              // on a 6x6 diagonal matrix

            double measure[6] = {gps_rel_pos[0], gps_rel_pos[1], gps_rel_pos[2], gps_ned_velocity[0], gps_ned_velocity[1], gps_ned_velocity[2]};

            for (int i = 0; i < 6; i++)
            {
                measure_v.push_back(measure[i]); // measure vector after processing
            }                                    // 1x6 vector
        }

        if (imu_available)
        {                                               // if *_available add the data to the csv,
            print_data_csv("../test_data/acc_input.csv", acc_input); // otherwhise add the default NaN vectors.
            acc_input.clear();
        }
        else
            print_data_csv("../test_data/acc_input.csv", no_acc_input);

        if (gps_available)
        {
            print_data_csv("../test_data/kalman_boat_R.csv", kalman_boat_R);
            print_data_csv("../test_data/measure.csv", measure_v);
            kalman_boat_R.clear();
            measure_v.clear();
        }
        else
        {
            print_data_csv("../test_data/kalman_boat_R.csv", no_R_no_measure);
            print_data_csv("../test_data/measure.csv", no_R_no_measure);
        }
    }

    return 0;
}

void printdata(vector<vector<string>> data)
{
    for (int i = 0; i < data.size(); i++)
    {
        for (int j = 0; j < data[i].size(); j++)
        {
            cout << data[i][j] << " ";
        }
        cout << endl;
    }
}

inline double toRadians(double value)
{
    return value * M_PI / 180.0;
}

int print_data_csv(string filename, vector<double> data)
{
    ofstream outfile(filename, ios::app); // open "filename" csv file in append mode
    if (!outfile.is_open())
    {
        cerr << "Error opening file!" << endl;
        return 1;
    }
    for (int i = 0;  i < data.size()-1; i++)
    {
        outfile << data[i] << ",";
    }
    outfile << data.back() << "\n";
    outfile.close();

    return 0;
}