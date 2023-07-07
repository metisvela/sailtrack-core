/** Libraries **/
// standard
#include <iostream>
#include <thread>
#include <mutex>
#include <chrono>

// third party
#include "json.hpp" // nlohmann/json library
using json = nlohmann::json;
//extern "C"
//{
#include <mosquitto.h>
//}

// custom
#include "fixed-kalman-filter.h"
#include "dynamic-kalman-filter.h"
#include "eigen-2-csv.h"
#include "model.h"
#include "preprocessing-logged.h"

/** Constants **/
constexpr double deg2rad = M_PI / 180.0;
constexpr double rad2deg = 1.0 / deg2rad;
constexpr double earth_circumference_meters = 40075 * 1000;

/** Config **/
constexpr auto mqtt_client_id = "sailtrack-filter_boat";
constexpr auto mqtt_pw = "sailtrack";

constexpr int kalman_sample_time_ms = (int) (kf::kalman_sample_time_s * 1000);
constexpr int mqtt_publish_interval_ms = 200;

/** Global data **/
GPS gps_ref{};            //
bool gps_ref_set = false; // set exactly once, no mutex necessary

/** Shared data **/
// GPS
GPS gps_data_shared{};
bool gps_data_shared_new = false;
std::mutex gps_mutex{};

// IMU
IMU imu_data_shared{};
bool imu_data_shared_new = false;
std::mutex imu_mutex{};

// ESTIMATES/TO PUBLISH
kf::fVector<6> xhat_shared{};
ORIENTATION orientation_shared{};
double lat_factor_shared{};
std::mutex estimate_mutex{};

/** Helper Functions **/
template <class T>
inline T square(T x) { return x * x; }

template <class T>
inline T angle_wrap_180(T angle)
{
    if (angle > 180)
        angle -= 360;
    return angle;
}

// raw data to kalman filter inputs
inline kf::fVector<3> imu2accel(IMU imu)
{
    return kf::fVector<3>{{imu.linearAccel_x}, {imu.linearAccel_y}, {imu.linearAccel_z}};
}

inline kf::fVector<6> gps2meas(GPS curr, GPS ref, double lat_factor)
{
    return kf::fVector<6>{
        // relative position
        {(curr.lat - ref.lat) * earth_circumference_meters / 360.0},
        {(curr.lon - ref.lon) * earth_circumference_meters * lat_factor / 360.0},
        {curr.hMSL - ref.hMSL},

        // speed
        {curr.velN},
        {curr.velE},
        {-curr.velD}};
}

inline kf::fMatrix<6, 6> gps2R(GPS gps)
{
    // We construct diagonal R
    return 0.25 * (kf::fVector<6>{square<double>(gps.hAcc), square<double>(gps.hAcc), square<double>(gps.vAcc),
                                  square<double>(gps.sAcc), square<double>(gps.sAcc), square<double>(gps.sAcc)})
                      .asDiagonal();
}

inline double gps2LatFactor(GPS curr, GPS ref)
{
    return std::cos(0.5 * (curr.lat + ref.lat) * deg2rad);
}

kf::fMatrix<3, 3> orientation_2_w2b(ORIENTATION orient)
{
    const double angle{deg2rad * orient.heading};
    return kf::fMatrix<3, 3>{
        {std::cos(angle), std::sin(angle), 0},
        {-std::sin(angle), std::cos(angle), 0},
        {0, 0, 1}};
}

// preprocessing helpers
void gps_preprocess(GPS &gps)
{
    gps.lat *= 1e-7;
    gps.lon *= 1e-7;
    gps.hMSL *= 1e-3;

    gps.velN *= 1e-3;
    gps.velE *= 1e-3;
    gps.velD *= 1e-3;

    gps.hAcc *= 1e-3;
    gps.vAcc *= 1e-3;
    gps.sAcc *= 1e-3;
}

void imu_preprocess(IMU &imu, ORIENTATION &orient)
{
    orient.heading = 360 - imu.euler_z;
    orient.pitch = -imu.euler_y;
    orient.roll = imu.euler_x;

    // Rotate linear acceleration from Body Reference Frame (BRF) to World Reference Frame (WRT)
    const kf::fMatrix<3, 3> world2body_mtx{orientation_2_w2b(orient)};

    const kf::fVector<3> linear_accel_wrf{world2body_mtx * imu2accel(imu)};

    imu.linearAccel_x = linear_accel_wrf(0);
    imu.linearAccel_y = linear_accel_wrf(1);
    imu.linearAccel_z = linear_accel_wrf(2);
}

/** Callbacks **/

// on connection subscribe to relevant topics
void on_connect_callback(struct mosquitto *mosq, void *obj, int rc)
{
    std::cout << "In connect callback..." << std::endl;
    if (rc == 0)    // check for connection
    {
        std::cout << "Subscribing to topics..." << std::endl;
        mosquitto_subscribe(mosq, NULL, "sensor/gps0", 0);
        mosquitto_subscribe(mosq, NULL, "sensor/imu0", 0);
    }
}  

// handle incoming gps message
void on_message_callback_gps(void *msg) // msg should be serialized json package
{
    GPS gps_data{};
    json gps_msg = json::parse((char *)msg);

    /* Translate json to C++ struct */
    gps_data.lat = gps_msg["lat"];
    gps_data.lon = gps_msg["lon"];
    gps_data.hMSL = gps_msg["hMSL"];

    gps_data.velN = gps_msg["velN"];
    gps_data.velE = gps_msg["velE"];
    gps_data.velD = gps_msg["velD"];

    gps_data.hAcc = gps_msg["hAcc"];
    gps_data.vAcc = gps_msg["vAcc"];
    gps_data.sAcc = gps_msg["sAcc"];

    if (!gps_ref_set)
    {
        gps_ref = gps_data;
        gps_ref_set = true;
    }

    /* save raw data in shared */
    { // Protected Access to shared data
        std::lock_guard<std::mutex> guard(gps_mutex);

        gps_data_shared = gps_data;
        gps_data_shared_new = true;
    }
}

// handle incoming imu message
void on_message_callback_imu(void *msg) // msg should be serialized json package
{
    IMU imu_data{};
    json imu_msg = json::parse((char *)msg);

    imu_data.euler_x = imu_msg["euler"]["x"];
    imu_data.euler_y = imu_msg["euler"]["y"];
    imu_data.euler_z = imu_msg["euler"]["z"];

    imu_data.linearAccel_x = imu_msg["linearAccel"]["x"];
    imu_data.linearAccel_y = imu_msg["linearAccel"]["y"];
    imu_data.linearAccel_z = imu_msg["linearAccel"]["z"];

    { // protected write to shared data
        std::lock_guard<std::mutex> guard(imu_mutex);
        imu_data_shared = imu_data;
        imu_data_shared_new = true;
    }
}

// handle message depending on topic
void on_message_callback(struct mosquitto *mosq, void *obj, const struct mosquitto_message *msg)
{
    if (strcmp(msg->topic, "sensor/gps0") == 0)
    {
        on_message_callback_gps(msg->payload);
    }
    else if (strcmp(msg->topic, "sensor/imu0") == 0)
    {
        on_message_callback_imu(msg->payload);
    }
}

/** Threads **/

void kalman_filter_thread(void)
{
    // init kalman filter
    kf::FixedKalmanFilter<6, 3, 6> fFilter(kf::fF, kf::fG, kf::fH, kf::fQ, kf::fx_init, kf::fP_init);

    // To be read from shared data
    GPS gps_data{};
    IMU imu_data{};
    bool gps_available;
    bool imu_available;

    // To be written to shared data
    double lat_factor{1.0};
    ORIENTATION orient{};

    // Run periodically
    auto next_wake_up_time = std::chrono::system_clock::now();
    while (true)
    {
        next_wake_up_time += std::chrono::milliseconds(kalman_sample_time_ms);

        /* Read data from shared, save in local vars*/
        { // protected read from shared data
            std::lock_guard<std::mutex> guard(gps_mutex);
            gps_available = gps_data_shared_new;
            gps_data = gps_data_shared;
            gps_data_shared_new = false;
        }

        { // protected read from shared data
            std::lock_guard<std::mutex> guard(imu_mutex);
            imu_available = imu_data_shared_new;
            imu_data = imu_data_shared;
            imu_data_shared_new = false;
        }

        /* Preprocess */
        // TODO: Check if this part can go wrong at launch (when no data is arriving yet)
        gps_preprocess(gps_data);                      // overwrites gps_data
        lat_factor = gps2LatFactor(gps_data, gps_ref); // determined from preprocessed gps_data
        imu_preprocess(imu_data, orient);              // input: imu_data - writes to: orient, overwrites imu_data

        /* Run Filter */
        // Determine acceleration and run prediction
        if (imu_available)
        {
            fFilter.predict(imu2accel(imu_data));
        }
        else
        {
            fFilter.predict(kf::fVector<3>::Zero()); // if imu data not given, assume speed stays the same (momentum conservation)
        }

        // If there is a new measurement, run filter/correction
        if (gps_available)
        {
            const kf::fVector<6> measurement{gps2meas(gps_data, gps_ref, lat_factor)};
            const kf::fMatrix<6, 6> R{gps2R(gps_data)};

            fFilter.update_noise_covariance(R);
            fFilter.correct(measurement);
        }

        /* Save current estimate in shared*/

        { // protected write to shared data
            std::lock_guard<std::mutex> guard(estimate_mutex);
            xhat_shared = fFilter.state_estimate();
            lat_factor_shared = lat_factor;
            orientation_shared = orient;
        }

        std::this_thread::sleep_until(next_wake_up_time);
    }
}

// Publishing thread, periodic (frequency independent of KF)
void mqtt_publish_thread(struct mosquitto *mosq)
{
    // to read from shared
    kf::fVector<6> xhat{};
    double lat_factor;
    ORIENTATION orientation;

    // sent data
    double sog;
    double cog;
    double lat;
    double lon;
    double altitude;
    double drift;

    // for debug messsage
    unsigned long iter{0};

    // Run periodically
    auto next_wake_up_time = std::chrono::system_clock::now();
    while (true)
    {
        next_wake_up_time += std::chrono::milliseconds(mqtt_publish_interval_ms);

        if (!gps_ref_set) // publish only if gps_ref set! Else values don't make sense
        {
            // reduce debug print freq in order not to spam
            iter++;
            if(iter%16 == 0)
                std::cout << "Waiting for GPS signal..." << std::endl;
            
            std::this_thread::sleep_until(next_wake_up_time);
            continue;
        }

        { // protected read from shared data
            std::lock_guard<std::mutex> guard(estimate_mutex);
            xhat = xhat_shared;
            lat_factor = lat_factor_shared;
            orientation = orientation_shared;
        }

        kf::fMatrix<3, 3> w2b_matrix = orientation_2_w2b(orientation);
        kf::fVector<3> pos = xhat(Eigen::seq(0, 2)); // position estimate
        kf::fVector<3> vel = xhat(Eigen::seq(3, 5)); // vel estimate

        // translation to desired data
        sog = std::sqrt(square<double>(vel(0)) + square<double>(vel(1)));
        cog = std::atan2(vel(1), vel(0));
        lat = pos(0) * 360.0 / earth_circumference_meters / lat_factor + gps_ref.lat;
        lon = pos(1) * 360.0 / earth_circumference_meters + gps_ref.lon;
        altitude = pos(2) + gps_ref.hMSL;

        // drift calculation
        drift = 0;
        if (sog > 0.5)
        {
            double cog_180 = angle_wrap_180(cog);
            double head_180 = angle_wrap_180(orientation.heading);
            drift = std::abs(head_180 - cog_180);
            if (head_180 > cog_180)
                drift = -drift;
        }

        /* Postprocessing Wind - Not implemented
            boat_xy_speed = w2b_matrix @ vel;
            twx = kalman_wind.x[0] - boat_xy_speed[0]
            twy = kalman_wind.x[1] - boat_xy_speed[1]
            tw_xy = np.array([twx.item(), twy.item(), 0], ).reshape(3, 1)
            twx = tw_xy[0]
            twy = tw_xy[1]
            tws = math.sqrt(twx ** 2 + twy ** 2) * MPS_TO_KNOTS_MULTIPLIER
            twa = math.degrees(math.atan2(twy, tws))
            twd = np.mod(orientation["heading"] + twa, 360)
            twd = twd if twd > 0 else 360 - twd
        */

        json package =
            {
                {"lon", lon},
                {"lat", lat},
                {"cog", cog},
                {"sog", sog},
                {"altitude", altitude},
                {"ascensionSpeed", vel(2)},
                {"heading", orientation.heading},
                {"pitch", angle_wrap_180(orientation.pitch)},
                {"roll", angle_wrap_180(orientation.roll)},
                {"drift", drift}
            };
        auto payload = package.dump();

        mosquitto_publish(mosq, NULL, "boat", payload.length(), payload.data(), 0, false);
        /* Publishing - previous python code
            mqtt.publish("boat", json.dumps({
                "lon": lon,
                "lat": lat,
                "cog": cog,
                "sog": sog,
                "altitude": altitude,
                "ascensionSpeed": neu_speed[2].item(),
                "heading": orientation["heading"],
                "pitch": angle_wrap_180(orientation["pitch"]),
                "roll": angle_wrap_180(orientation["roll"]),
                "drift": drift
            }))
        */

        std::this_thread::sleep_until(next_wake_up_time);
    }
}

int main(void)
{
    std::cout << "Entered main..." << std::endl;
    
    /* Set up MQTT Connection*/
    int rc;
    struct mosquitto *mosq;

    mosquitto_lib_init();

    mosq = mosquitto_new(mqtt_client_id, false, NULL);
    mosquitto_threaded_set(mosq, true);
    mosquitto_username_pw_set(mosq, "mosquitto", mqtt_pw);

    mosquitto_connect_callback_set(mosq, on_connect_callback); // subscribe to relevant topics
    mosquitto_message_callback_set(mosq, on_message_callback); // handles messages for all topics

    rc = mosquitto_connect(mosq, "192.168.42.1", 1883, 10); // 192.168.42.1
    if (rc != 0)
    {
        // printf("Client could not connect to broker! Error Code: %d\n", rc);
        std::cout << "Error on mosquitto_connect call" << std::endl;
        mosquitto_destroy(mosq);
        return -1;
    }

    std::cout << "Before mosquitto loop start..." << std::endl;
    mosquitto_loop_start(mosq); // starts a seperate mqtt networking thread
    /* mqtt - previous python code
    mqtt = Client(MQTT_CLIENT_ID)
    mqtt.username_pw_set("mosquitto", os.environ["SAILTRACK_GLOBAL_PASSWORD"])
    mqtt.message_callback_add("sensor/gps0", on_message_callback_gps)
    mqtt.message_callback_add("sensor/imu0", on_message_callback_imu)
    mqtt.connect("localhost")
    mqtt.subscribe("sensor/gps0")
    mqtt.subscribe("sensor/imu0")
    mqtt.loop_start()
    */
   
    /* Start threads */
    std::cout << "Starting threads..." << std::endl;
    std::thread tKalman(kalman_filter_thread);
    std::thread tPublish(mqtt_publish_thread, mosq);

    std::cout << "End of setup reached..." << std::endl;

    tKalman.join();
    tPublish.join();

    /* clean-up */
    mosquitto_disconnect(mosq);
    mosquitto_destroy(mosq);
    mosquitto_lib_cleanup();

    std::cout << "End of main reached, SHOULD NEVER ARRIVE HERE" << std::endl;
}