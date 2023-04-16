#include <iostream>
#include <fstream>
#include <sstream>
#include <vector>
#include <string>
#include <cmath>
#include <stdexcept>
#include <limits>

#include "preprocessing-logged.h"

using namespace std;

// Constants
constexpr double EARTH_CIRCUMFERENCE_METERS{40075.0 * 1000.0};
constexpr double MPS_TO_KNOTS_MULTIPLIER{1.94384};

// Structures will hold current sensor data
IMU imu_data{};
ORIENTATION orientation{};
GPS gps_data{}, gps_ref{};

constexpr double nanValue = numeric_limits<double>::quiet_NaN();                                         //  default values printed in the
const vector<double> no_acc_input = {nanValue, nanValue, nanValue};                                  //  csv when data not available
const vector<double> no_R_no_measure = {nanValue, nanValue, nanValue, nanValue, nanValue, nanValue}; //

// helper functions
inline double toRadians(double value) { return value * M_PI / 180.0; }
int vector2csv(string, vector<double>);

// Function takes raw sensor csv-file,
// creates a 3 csv-files with preprocessed acceleration, measurment and noise covariance data.
int preprocess_logged_data(string filename)
{
    double LAT_FACTOR = 1;

    bool gps_available = true; // Control flow flags, set to false by exception generated from stod() function.
    bool imu_available = true;
    bool gps_ref_set = false;

    vector<double> acc_input;     // current preprocessed data line to be written to csv
    vector<double> kalman_boat_R; //
    vector<double> measure_v;     //

    vector<vector<string>> data; // var to save raw data file
    string line;

    remove("test_data/acc_input.csv");     // remove previous version of the output files
    remove("test_data/kalman_boat_R.csv"); //
    remove("test_data/measure.csv");       //

    // Read raw data into data vector, if no data is available saves an empty string.
    ifstream file(filename);
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

    // Preprocess each row/time stamp
    for (int i = 1; i < data.size(); i++)
    {
        // Try to read imu data
        try
        {
            // "no wind" raw data mapping
            imu_data.euler_x = stod(data[i][3]);
            imu_data.euler_y = stod(data[i][4]);
            imu_data.euler_z = stod(data[i][5]);
            imu_data.linearAccel_x = stod(data[i][13]);
            imu_data.linearAccel_y = stod(data[i][14]);
            imu_data.linearAccel_z = stod(data[i][15]);

            orientation.heading = 360 - imu_data.euler_z;
            orientation.pitch = -imu_data.euler_y;
            orientation.roll = imu_data.euler_x;

            imu_available = true;
        }
        // No data (empty string) triggers stod invalid argument exception.
        catch (const invalid_argument &e)
        {
            imu_available = false; // at least one of imu metrics is not available
        }

        // Try to read gps data
        try
        {
            //  "no wind" raw data mapping
            gps_data.vAcc = stod(data[i][18]) * pow(10, -3);
            gps_data.hAcc = stod(data[i][8]) * pow(10, -3);
            gps_data.sAcc = stod(data[i][17]) * pow(10, -3);
            gps_data.headAcc = stod(data[i][10]);
            gps_data.lon = stod(data[i][16]) * pow(10, -7);
            gps_data.lat = stod(data[i][12]) * pow(10, -7);
            gps_data.hMSL = stod(data[i][9]) * pow(10, -3);
            gps_data.velN = stod(data[i][21]) * pow(10, -3);
            gps_data.velE = stod(data[i][20]) * pow(10, -3);
            gps_data.velD = stod(data[i][19]) * pow(10, -3);
            gps_data.gSpeed = stod(data[i][7]);
            gps_data.headMot = stod(data[i][11]);
            gps_data.fixType = stod(data[i][6]);
            gps_data.epoch = stod(data[i][2]);

            gps_available = true;
        }
        // No data (empty string) triggers stod invalid argument exception.
        catch (const invalid_argument &e)
        {
            gps_available = false; // at least one of gps metrics is not available
        }

        // If available, prerocess imu data
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

        // If available, preprocess gps data
        if (gps_available)
        {
            // Set gps reference the first time data is available (done once)
            if (!gps_ref_set)
            {
                gps_ref = gps_data;
                gps_ref_set = true;
            }

            LAT_FACTOR = cos(toRadians(gps_data.lat + gps_ref.lat) / 2);

            double gps_rel_pos[3] = {(gps_data.lat - gps_ref.lat) * EARTH_CIRCUMFERENCE_METERS / 360,
                                     (gps_data.lon - gps_ref.lon) * EARTH_CIRCUMFERENCE_METERS * LAT_FACTOR / 360,
                                     (gps_data.hMSL - gps_ref.hMSL)};

            double gps_ned_velocity[3] = {gps_data.velN, gps_data.velE, -gps_data.velD};
            
            // Sensor accuracy readings used.
            // Assume uncorrelated noise and that accuracy readings give 2 * std deviation interval. (0.95 confidence interval of gaussian)
            double local_R[6] = {pow(gps_data.hAcc, 2) * 0.25, pow(gps_data.hAcc, 2) * 0.25, pow(gps_data.vAcc, 2) * 0.25,
                                 pow(gps_data.sAcc, 2) * 0.25, pow(gps_data.sAcc, 2) * 0.25, pow(gps_data.sAcc, 2) * 0.25};

            for (int i = 0; i < 6; i++)
            {
                kalman_boat_R.push_back(local_R[i]); // R matrix after preprocessing. These six elements are the element
            }                                        // on a 6x6 diagonal matrix

            double measure[6] = {gps_rel_pos[0], gps_rel_pos[1], gps_rel_pos[2], gps_ned_velocity[0], gps_ned_velocity[1], gps_ned_velocity[2]};

            for (int i = 0; i < 6; i++)
            {
                measure_v.push_back(measure[i]); // measure vector after processing
            }                                    // 1x6 vector
        }

        // Write preprocessed time stamp into seperate files
        if (imu_available)
        {                                                     // if *_available add the data to the csv,
            vector2csv("test_data/acc_input.csv", acc_input); // otherwhise add the default NaN vectors.
            acc_input.clear();
        }
        else
            vector2csv("test_data/acc_input.csv", no_acc_input);

        if (gps_available)
        {
            vector2csv("test_data/kalman_boat_R.csv", kalman_boat_R);
            vector2csv("test_data/measure.csv", measure_v);
            kalman_boat_R.clear();
            measure_v.clear();
        }
        else
        {
            vector2csv("test_data/kalman_boat_R.csv", no_R_no_measure);
            vector2csv("test_data/measure.csv", no_R_no_measure);
        }
    }

    return 0;
}


int vector2csv(string filename, vector<double> data)
{
    ofstream outfile(filename, ios::app); // open "filename" csv file in append mode
    if (!outfile.is_open())
    {
        cerr << "Error opening file!" << endl;
        return 1;
    }
    for (int i = 0; i < data.size() - 1; i++)
    {
        outfile << data[i] << ",";
    }
    outfile << data.back() << "\n";
    outfile.close();

    return 0;
}