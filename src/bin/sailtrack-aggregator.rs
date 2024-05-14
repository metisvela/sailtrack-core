use eskf::ESKF;
use std::intrinsics::drop_in_place;
use std::slice::from_raw_parts;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use nalgebra::{DMatrix, DVector, Matrix2, Matrix3, Point3, Vector2, Vector3};
use rumqttc::{Client, Event, Incoming, MqttOptions, QoS};
use serde::{Deserialize, Serialize};

// Connection parameters
const MQTT_PUBLISH_FREQ_HZ: u64 = 5;

// Kalman filter parameters
const MPS_TO_KNTS_MULTIPLIER: f32 = 1.94384;
const EARTH_CIRCUMFERENCE_METERS: f32 = 40075.0 * 1000.0;
const KALMAN_SAMPLE_TIME_MS: u64 = 200;
const LAT_FACTOR: f32 = 1.0;
const GPS_DEAD_VALUE: i32 = 5;

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
    ascension: f32,
    heading: f32,
    pitch: f32,
    roll: f32,
    drift: f32,
}

#[derive(Debug, Clone, Copy)]

struct Measure {
    data: Gps,
    gps_ref: Gps,
    new_measure: bool,
}

#[derive(Debug, Clone, Copy)]
struct Input {
    acceleration: Vector3<f32>,
    rotation: Vector3<f32>,
    new_input: bool,
}

// Function to continuously try locking the mutex until successful
fn acquire_lock<T>(mutex: &Arc<Mutex<T>>) -> std::sync::MutexGuard<T> {
    loop {
        if let Ok(guard) = mutex.try_lock() {
            return guard;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn get_matrix_from_measure(
    measure_arc: &Arc<Mutex<Measure>>,
) -> (
    Point3<f32>,
    Vector2<f32>,
    f32,
    Matrix3<f32>,
    Matrix2<f32>,
    f32,
) {
    let measure_lock = acquire_lock(&measure_arc);

    let position = Point3::new(
        (measure_lock.data.lat - measure_lock.gps_ref.lat) * EARTH_CIRCUMFERENCE_METERS / 360.0,
        (measure_lock.data.lon - measure_lock.gps_ref.lon)
            * EARTH_CIRCUMFERENCE_METERS
            * LAT_FACTOR
            / 360.0,
        measure_lock.data.h_msl - measure_lock.gps_ref.h_msl,
    );
    let velocity_xy = Vector2::new(measure_lock.data.vel_n, measure_lock.data.vel_e);
    let velocity_z = -measure_lock.data.vel_d;
    let mut pos_variance = Matrix3::zeros();
    pos_variance[(0, 0)] = 0.25 * measure_lock.data.h_acc.powf(2.0);
    pos_variance[(1, 1)] = 0.25 * measure_lock.data.h_acc.powf(2.0);
    pos_variance[(2, 2)] = 0.25 * measure_lock.data.h_acc.powf(2.0);
    let vel_variance = Matrix2::identity() * 0.25 * measure_lock.data.s_acc.powf(2.0);
    (
        position,
        velocity_xy,
        velocity_z,
        pos_variance,
        vel_variance,
        0.25 * measure_lock.data.s_acc.powf(2.0),
    )
}

// Function that keeps on controll if the GPS fix is obtained
fn wait_for_fix_tipe(measure_arc: &Arc<Mutex<Measure>>) -> () {
    loop {
        let measure_lock = acquire_lock(&measure_arc);
        if measure_lock.gps_ref.fix_type == 3 {
            drop(measure_lock);
            break;
        }
        drop(measure_lock);
        thread::sleep(Duration::from_millis(1000));
    }
}

fn on_message_imu(message: Imu, input: &mut Arc<Mutex<Input>>) {
    println!("Received IMU message: {message:?}");

    let accel = Vector3::new(
        message.linear_accel.x,
        message.linear_accel.y,
        message.linear_accel.z,
    );
    let orientation = Vector3::new(message.euler.x, -message.euler.y, 360.0 - message.euler.z);
    let mut input_lock = acquire_lock(input);
    input_lock.new_input = true;
    input_lock.acceleration = accel;
    input_lock.rotation = orientation;
}

fn on_message_gps(message: Gps, measure_mutex: &mut Arc<Mutex<Measure>>) {
    println!("Received GPS message: {message:?}");
    let mut measure_lock = acquire_lock(measure_mutex);
    measure_lock.data.lat = message.lat * f32::powf(10.0, -7.0);
    measure_lock.data.lon = message.lon * f32::powf(10.0, -7.0);
    measure_lock.data.h_msl = message.h_msl * f32::powf(10.0, -3.0);
    measure_lock.data.vel_n = message.vel_n * f32::powf(10.0, -3.0);
    measure_lock.data.vel_e = message.vel_e * f32::powf(10.0, -3.0);
    measure_lock.data.vel_d = message.vel_d * f32::powf(10.0, -3.0);
    measure_lock.data.h_acc = message.h_acc * f32::powf(10.0, -3.0);
    measure_lock.data.v_acc = message.v_acc * f32::powf(10.0, -3.0);
    measure_lock.data.s_acc = message.s_acc * f32::powf(10.0, -3.0);
    measure_lock.new_measure = true;
}

fn main() {
    // Defining structures and filter parameters
    let filter_ts = Duration::from_millis(KALMAN_SAMPLE_TIME_MS);

    let mut input = Input {
        acceleration: Vector3::new(0.0, 0.0, 0.0),
        rotation: Vector3::new(0.0, 0.0, 0.0),
        new_input: false,
    };

    let empty_gps_struct = Gps {
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

    let mut measure = Measure {
        data: empty_gps_struct,
        gps_ref: empty_gps_struct,
        new_measure: false,
    };

    // Creating ESKF object
    let mut filter = eskf::Builder::new()
        .acceleration_variance(0.01) // FIXME
        .rotation_variance(0.01) // FIXME
        .build();

    // Defining Mutex for thread share
    let measure_mutex = Arc::new(Mutex::new(measure));
    let input_mutex = Arc::new(Mutex::new(input));
    let filter_mutex = Arc::new(Mutex::new(filter));

    // TODO: Add username and password authentication
    let mqqt_opts = MqttOptions::new("sailtrack-aggregator", "localhost", 1883);

    let (client, mut connection) = Client::new(mqqt_opts, 10);
    client.subscribe("sensor/gps0", QoS::AtMostOnce).unwrap();
    client.subscribe("sensor/imu0", QoS::AtMostOnce).unwrap();

    // MQTT Callbacks thread
    let mut measure_clone = Arc::clone(&measure_mutex);
    let mut input_clone = Arc::clone(&input_mutex);
    thread::spawn(move || {
        for notification in connection.iter().flatten() {
            if let Event::Incoming(Incoming::Publish(packet)) = notification {
                let topic = packet.topic.as_str().to_string(); // Clone the topic for later use

                match topic.as_str() {
                    "sensor/imu0" => {
                        let payload = packet.payload.clone(); // Clone the payload for later use
                        on_message_imu(
                            serde_json::from_slice(payload.as_ref()).unwrap(),
                            &mut input_clone,
                        );
                    }
                    "sensor/gps0" => {
                        let payload = packet.payload.clone(); // Clone the payload for later use
                        on_message_gps(
                            serde_json::from_slice(payload.as_ref()).unwrap(),
                            &mut measure_clone,
                        );
                    }
                    _ => (),
                }
            }
        }
    });

    // GPS fix check
    measure_clone = Arc::clone(&measure_mutex);
    thread::spawn(move || {
        println!("Waiting for GPS fix...");
        loop {
            let mut measure_lock = acquire_lock(&measure_clone);
            if measure_lock.data.fix_type == 3 {
                measure_lock.gps_ref = measure_lock.data;
                break; // Exit the loop when GPS fix is obtained
            }
            drop(measure_lock); // Ensure the lock is released
            thread::sleep(Duration::from_millis(1000));
        }
    });

    // Kalman filter thread
    let measure_clone = Arc::clone(&measure_mutex);
    let input_clone = Arc::clone(&input_mutex);
    let filter_clone = Arc::clone(&filter_mutex);
    thread::spawn(move || loop {

        // Check if the GPS fix has been obtained
        wait_for_fix_tipe(&measure_clone);

        let thread_start = Instant::now();
        let mut measure_lock = acquire_lock(&measure_clone);
        let mut input_lock = acquire_lock(&input_clone);
        let mut filter_lock = acquire_lock(&filter_clone);

        // Reliable GPS and IMU data: predict and update
        if measure_lock.new_measure {
            filter_lock.predict(input_lock.acceleration, input_lock.rotation, filter_ts);
            let (position, velocity_xy, velocity_z, pos_variance, vel_variance, vel_z_variance) =
                get_matrix_from_measure(&measure_clone);
            filter_lock
                .observe_position_velocity2d(position, pos_variance, velocity_xy, vel_variance)
                .unwrap();
            filter_lock
                .observe_height(velocity_z, vel_z_variance)
                .unwrap();
            measure_lock.new_measure = false;
            input_lock.new_input = false;
        }
        // Only reliable GPS data: only update
        else if !input_lock.new_input && measure_lock.new_measure {
            let (position, velocity_xy, velocity_z, pos_variance, vel_variance, vel_z_variance) =
                get_matrix_from_measure(&measure_clone);
            filter_lock
                .observe_position_velocity2d(position, pos_variance, velocity_xy, vel_variance)
                .unwrap();
            filter_lock
                .observe_height(velocity_z, vel_z_variance)
                .unwrap();
            measure_lock.new_measure = false;
        }
        // Only reliable or old IMU data: only predict
        else {
            filter_lock.predict(input_lock.acceleration, input_lock.rotation, filter_ts);
            input_lock.new_input = false;
        }
        drop(input_lock);
        drop(measure_lock);
        drop(filter_lock);
        let elapsed = thread_start.elapsed();
        if elapsed < filter_ts {
            thread::sleep(filter_ts - elapsed);
        }
    });

    //MQTT publish thread
    let filter_clone = Arc::clone(&filter_mutex);
    let measure_clone = Arc::clone(&measure_mutex);
    thread::spawn(move || loop {
        // Check if the GPS fix has been obtained
        wait_for_fix_tipe(&measure_clone);
        let filter_lock = acquire_lock(&filter_clone);
        let position = filter_lock.position;
        let velocity = filter_lock.velocity;
        let quat_orient = filter_lock.orientation;
        let euler_orient = quat_orient.euler_angles();

        let sog = velocity.norm();
        let cog = f32::atan2(velocity.y, velocity.x).to_degrees();

        let roll = euler_orient.0;
        let pitch = euler_orient.1;
        let heading = euler_orient.2;

        let body_to_world_mtx = Matrix3::new(
            f32::cos(heading),
            f32::sin(heading),
            0.0,
            -f32::sin(heading),
            f32::cos(heading),
            0.0,
            0.0,
            0.0,
            1.0,
        );

        let ned_pos = body_to_world_mtx * position;

        let message = Boat {
            lon: 0.0,
            lat: 0.0,
            cog: cog,
            sog: sog,
            altitude: 0.0,
            ascension: 0.0,
            heading: heading,
            pitch: pitch,
            roll: roll,
            drift: 0.0,
        };
        client
            .publish(
                "boat",
                QoS::AtLeastOnce,
                false,
                serde_json::to_vec(&message).unwrap(),
            )
            .unwrap();
        println!("Published boat measurement: {message:?}");
        thread::sleep(Duration::from_millis(1000 / MQTT_PUBLISH_FREQ_HZ));
    });
}
