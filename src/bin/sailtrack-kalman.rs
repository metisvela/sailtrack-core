use kalmanfilt::kalman::kalman_filter::KalmanFilter as Kalman;
use rand::Rng;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use nalgebra::{OMatrix, OVector, U3, U6};
use rumqttc::{Client, Event, Incoming, MqttOptions, QoS};
use serde::{Deserialize, Serialize};

// Connection parameters
const MQTT_PUBLISH_FREQ_HZ: u64 = 5;

// Kalman filter parameters
const MPS_TO_KNTS_MULTIPLIER: f32 = 1.94384;
const EARTH_CIRCUMFERENCE_METERS: f32 = 40075.0 * 1000.0;
const KALMAN_SAMPLE_TIME_MS: u64 = 200;
const LAT_FACTOR: f32 = 1.0;

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "camelCase")]
struct Euler {
    x: f32,
    y: f32,
    z: f32,
}
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "camelCase")]
struct LinearAccel {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct Imu {
    euler: Euler,
    linear_accel: LinearAccel,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "camelCase")]
struct Gps {
    fix_type: i32,
    epoch: i64,
    lon: f32,
    lat: f32,
    #[serde(rename = "hMSL")]
    h_msl: f32,
    h_acc: f32,
    v_acc: f32,
    vel_n: f32,
    vel_e: f32,
    vel_d: f32,
    g_speed: f32,
    head_mot: f32,
    s_acc: f32,
    head_acc: f32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Boat {
    lon: f32,
    lat: f32,
    cog: f32,
    sog: f32,
    altitude: f32,
    ascension_speed: f32,
    heading: f32,
    pitch: f32,
    roll: f32,
    drift: f32,
}

#[derive(Debug, Clone, Copy)]
struct Measure {
    meas: OVector<f32, U6>,
    meas_variance: OMatrix<f32, U6, U6>,
    new_measure: bool,
}

#[derive(Debug, Clone, Copy)]
struct Input {
    acceleration: OVector<f32, U3>,
    orientation: OVector<f32, U3>,
    new_input: bool,
}

// Function to continuously try locking the mutex until successful
fn acquire_lock<T>(mutex: &Arc<Mutex<T>>) -> std::sync::MutexGuard<T> {
    loop {
        if let Ok(guard) = mutex.try_lock() {
            return guard;
        }
        let mut rng = rand::thread_rng();
        let sleep_time: u64 = rng.gen_range(5..10);
        thread::sleep(Duration::from_millis(sleep_time));
    }
}

// Function to compute the measure for the Kalman filter from the raw GPS data
fn get_measure_forom_gps(gps_data: &Gps, reference: &Gps) -> Measure {
    let meas_vec = vec![
        (gps_data.lat * f32::powf(10.0, -7.0) - reference.lat * f32::powf(10.0, -7.0))
            * EARTH_CIRCUMFERENCE_METERS
            / 360.0,
        (gps_data.lon * f32::powf(10.0, -7.0) - reference.lon * f32::powf(10.0, -7.0))
            * EARTH_CIRCUMFERENCE_METERS
            * LAT_FACTOR
            / 360.0,
        gps_data.h_msl * f32::powf(10.0, -3.0) - reference.h_msl * f32::powf(10.0, -3.0),
        gps_data.vel_n * f32::powf(10.0, -3.0),
        gps_data.vel_e * f32::powf(10.0, -3.0),
        gps_data.vel_d * f32::powf(10.0, -3.0),
    ];

    let meas: OVector<f32, U6> = OVector::<f32, U6>::from_iterator(meas_vec);

    let accuracy_penality_factor = 100.0;
    let mut meas_variance: OMatrix<f32, U6, U6> = OMatrix::zeros_generic(U6, U6);
    let acc_scaling = f32::powf(10.0, -3.0);

    for i in 0..5 {
        let acc_value = match i {
            0 | 1 => gps_data.h_acc,
            2 => gps_data.v_acc,
            _ => gps_data.s_acc,
        };
        meas_variance[(i, i)] = 0.25 * acc_value.powf(2.0) * acc_scaling;
        if gps_data.fix_type != 3 {
            meas_variance[(i, i)] *= accuracy_penality_factor;
        }
    }

    Measure {
        meas,
        meas_variance,
        new_measure: true,
    }
}

// Function that keeps on controll if the GPS fix is obtained
fn wait_for_fix_tipe(gps_ref_arc: &Arc<Mutex<Gps>>) -> bool {
    let gps_ref_lock = acquire_lock(gps_ref_arc);
    if gps_ref_lock.fix_type == 3 {
        drop(gps_ref_lock);
        return true;
    }
    drop(gps_ref_lock);
    loop {
        let gps_ref_lock = acquire_lock(gps_ref_arc);
        if gps_ref_lock.fix_type == 3 {
            drop(gps_ref_lock);
            return true;
        }
        drop(gps_ref_lock);
        thread::sleep(Duration::from_millis(1000));
    }
}

fn on_message_imu(message: Imu, input: &Arc<Mutex<Input>>) {
    let accel_vec = vec![
        message.linear_accel.x,
        message.linear_accel.y,
        message.linear_accel.z,
    ];
    let accel = OVector::<f32, U3>::from_iterator(accel_vec);
    let orient_vec = vec![message.euler.x, -message.euler.y, 360.0 - message.euler.z];
    let orient = OVector::<f32, U3>::from_iterator(orient_vec);
    let mut input_lock = acquire_lock(input);
    input_lock.new_input = true;
    input_lock.acceleration = accel;
    input_lock.orientation = orient;
    drop(input_lock);
}

fn on_message_gps(message: Gps, gps_ref_arc: &Arc<Mutex<Gps>>, measure_arc: &Arc<Mutex<Measure>>) {
    let mut gps_ref_lock = acquire_lock(gps_ref_arc);
    let mut measure_lock = acquire_lock(measure_arc);

    if gps_ref_lock.fix_type != 3 {
        *gps_ref_lock = message;
    }
    let measure = get_measure_forom_gps(&message, &gps_ref_lock);
    *measure_lock = measure;
    drop(measure_lock);
    drop(gps_ref_lock);
}

// Kalman predict function on new input
fn filter_predict(kalman: &mut Kalman<f32, U6, U6, U3>, input: &mut Input) {
    kalman.predict(Some(&input.acceleration), None, None, None);
    input.new_input = false;
}

// Kalman update function on new measure
fn filter_update(kalman: &mut Kalman<f32, U6, U6, U3>, measure: &mut Measure) {
    kalman
        .update(&measure.meas, Some(&measure.meas_variance), None)
        .unwrap();
    measure.new_measure = false;
}

fn angle_wrap_180(angle: f32) -> f32 {
    (angle + 180.0) % 360.0 - 180.0
}

fn angle_unwrap(angle: f32) -> f32 {
    let unwrapped_angle = angle % 360.0;
    if unwrapped_angle < 0.0 {
        unwrapped_angle + 360.0
    } else {
        unwrapped_angle
    }
}

fn main() {
    // Defining structures and filter parameters
    let filter_ts = Duration::from_millis(KALMAN_SAMPLE_TIME_MS);

    let input = Input {
        acceleration: OVector::<f32, U3>::zeros(),
        orientation: OVector::<f32, U3>::zeros(),
        new_input: false,
    };

    let gps_ref = Gps {
        fix_type: 0,
        epoch: 0,
        lon: 0.0,
        lat: 0.0,
        h_msl: 0.0,
        h_acc: 0.0,
        v_acc: 0.0,
        vel_n: 0.0,
        vel_e: 0.0,
        vel_d: 0.0,
        g_speed: 0.0,
        head_mot: 0.0,
        s_acc: 0.0,
        head_acc: 0.0,
    };

    let measure = Measure {
        meas: OVector::<f32, U6>::zeros(),
        meas_variance: OMatrix::<f32, U6, U6>::identity(),
        new_measure: false,
    };

    // Creating ESKF object
    let w_std = 0.01;
    let sample_time = filter_ts.as_secs_f32();
    let transition_mtx = OMatrix::<f32, U6, U6>::from_column_slice(&[
        1.0,
        0.0,
        0.0,
        sample_time,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        sample_time,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        sample_time,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ]);
    let input_mtx = OMatrix::<f32, U6, U3>::from_row_slice(&[
        sample_time.powi(2) / 2.0,
        0.0,
        0.0,
        0.0,
        sample_time.powi(2) / 2.0,
        0.0,
        0.0,
        0.0,
        sample_time.powi(2) / 2.0,
        sample_time,
        0.0,
        0.0,
        0.0,
        sample_time,
        0.0,
        0.0,
        0.0,
        sample_time,
    ]);
    let output_mtx = OMatrix::<f32, U6, U6>::identity();
    let noise_state_cov = input_mtx * input_mtx.transpose() * w_std;
    let noise_meas_cov = OMatrix::<f32, U6, U6>::identity();

    let filter = Kalman::<f32, nalgebra::Const<6>, nalgebra::Const<6>, nalgebra::Const<3>> {
        F: transition_mtx,
        H: output_mtx,
        B: Some(input_mtx),
        Q: noise_state_cov,
        R: noise_meas_cov,
        ..Default::default()
    };

    // Defining Mutex for thread share
    let gps_ref_mutex = Arc::new(Mutex::new(gps_ref));
    let measure_mutex = Arc::new(Mutex::new(measure));
    let input_mutex = Arc::new(Mutex::new(input));
    let filter_mutex = Arc::new(Mutex::new(filter));

    // TODO: Add username and password authentication
    let mut mqqt_opts = MqttOptions::new("sailtrack-kalman", "192.168.42.1", 1883);
    mqqt_opts.set_credentials("mosquitto", "sailtrack");

    let (client, mut connection) = Client::new(mqqt_opts, 10);
    client.subscribe("sensor/gps0", QoS::AtMostOnce).unwrap();
    client.subscribe("sensor/imu0", QoS::AtMostOnce).unwrap();

    // // MQTT Callbacks thread
    let gps_ref_clone = Arc::clone(&gps_ref_mutex);
    let measure_clone = Arc::clone(&measure_mutex);
    let input_clone = Arc::clone(&input_mutex);
    thread::spawn(move || {
        for notification in connection.iter().flatten() {
            if let Event::Incoming(Incoming::Publish(packet)) = notification {
                let topic = packet.topic.as_str().to_string(); // Clone the topic for later use
                match topic.as_str() {
                    "sensor/imu0" => {
                        let payload = packet.payload.clone(); // Clone the payload for later use
                        on_message_imu(
                            serde_json::from_slice(payload.as_ref()).unwrap(),
                            &input_clone,
                        );
                    }
                    "sensor/gps0" => {
                        let payload = packet.payload.clone(); // Clone the payload for later use
                        on_message_gps(
                            serde_json::from_slice(payload.as_ref()).unwrap(),
                            &gps_ref_clone,
                            &measure_clone,
                        );
                    }
                    _ => (),
                }
            }
        }
    });

    // Kalman filter thread
    let gps_ref_clone = Arc::clone(&gps_ref_mutex);
    let measure_clone = Arc::clone(&measure_mutex);
    let input_clone = Arc::clone(&input_mutex);
    let filter_clone = Arc::clone(&filter_mutex);
    thread::spawn(move || loop {
        // Check if the GPS fix has been obtained
        wait_for_fix_tipe(&gps_ref_clone);
        let thread_start = Instant::now();
        let mut measure_lock = acquire_lock(&measure_clone);
        let mut input_lock = acquire_lock(&input_clone);
        let mut filter_lock = acquire_lock(&filter_clone);

        match (measure_lock.new_measure, input_lock.new_input) {
            (true, true) => {
                filter_predict(&mut filter_lock, &mut input_lock);
                filter_update(&mut filter_lock, &mut measure_lock);
                drop(input_lock);
                drop(measure_lock);
                drop(filter_lock);
            }
            (true, false) => {
                filter_update(&mut filter_lock, &mut measure_lock);
                drop(measure_lock);
                drop(filter_lock);
            }
            _ => {
                filter_predict(&mut filter_lock, &mut input_lock);
                drop(input_lock);
                drop(filter_lock);
            }
        }
        let elapsed = thread_start.elapsed();
        if elapsed.as_millis() < filter_ts.as_millis() {
            thread::sleep(filter_ts - elapsed);
        }
    });

    //MQTT publish loop
    let gps_ref_clone = Arc::clone(&gps_ref_mutex);
    let input_clone = Arc::clone(&input_mutex);
    let filter_clone = Arc::clone(&filter_mutex);
    loop {
        // Check if the GPS fix has been obtained

        let input_lock = acquire_lock(&input_clone);
        let roll = input_lock.orientation.x;
        let pitch = input_lock.orientation.y;
        let heading = input_lock.orientation.z;
        drop(input_lock);

        wait_for_fix_tipe(&gps_ref_clone);
        let filter_lock = acquire_lock(&filter_clone);
        let position = filter_lock.x.fixed_rows::<3>(0);
        let velocity = filter_lock.x.fixed_rows::<3>(1);

        let sog = (velocity.x.powi(2) + velocity.y.powi(2)).sqrt() * MPS_TO_KNTS_MULTIPLIER;
        let mut cog = heading;
        let mut drift = 0.0;
        if sog > 1.0 {
            cog = f32::atan2(velocity.y, velocity.x).to_degrees();
            cog = angle_unwrap(cog);
            let cog_180 = angle_wrap_180(cog);
            let head_180 = angle_wrap_180(heading);
            drift = (head_180 - cog_180).abs();
            if head_180.abs() + cog_180.abs() > 180.0 {
                drift = 360.0 - drift;
            }
            if head_180 > cog_180 {
                drift = -drift;
            }
        }
        let gps_ref_lock = acquire_lock(&gps_ref_clone);
        let lat = position.x * 360.0 / EARTH_CIRCUMFERENCE_METERS / LAT_FACTOR
            + gps_ref_lock.lat * f32::powf(10.0, -7.0);
        let lon: f32 = position.y * 360.0 / EARTH_CIRCUMFERENCE_METERS
            + gps_ref_lock.lon * f32::powf(10.0, -7.0);
        let altitude = position.z + gps_ref_lock.h_msl * f32::powf(10.0, -3.0);
        drop(gps_ref_lock);
        let z_speed = velocity.z * MPS_TO_KNTS_MULTIPLIER;
        drop(filter_lock);

        let message = Boat {
            lon,
            lat,
            cog,
            sog,
            altitude,
            ascension_speed: z_speed,
            heading,
            pitch,
            roll,
            drift,
        };
        client
            .publish(
                "boat/kalman",
                QoS::AtLeastOnce,
                false,
                serde_json::to_vec(&message).unwrap(),
            )
            .unwrap();

        thread::sleep(Duration::from_millis(1000 / MQTT_PUBLISH_FREQ_HZ));
    }
}
