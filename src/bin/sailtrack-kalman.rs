use eskf::ESKF;
use rand::Rng;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use nalgebra::{Matrix2, Matrix3, Point3, Vector2, Vector3};
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
    position: Point3<f32>, 
    velocity_xy: Vector2<f32>, 
    velocity_z: f32,
    pos_variance: Matrix3<f32>, 
    vel_variance: Matrix2<f32>,
    vel_z_variance: f32,
    new_measure: bool,
}

#[derive(Debug, Clone, Copy)]
struct Input {
    acceleration: Vector3<f32>,
    rotation: Vector3<f32>,
    new_input: bool,
}

#[derive(Debug, Clone, Copy)]
struct Debug {
    mqtt_recieve_process_counter:u64,
    mqtt_send_process_counter:u64,
    filter_process_counter:u64,
}


// Function to continuously try locking the mutex until successful
fn acquire_lock<T>(mutex: &Arc<Mutex<T>>) -> std::sync::MutexGuard<T> {
    let mut count = 0;
    loop {
        if let Ok(guard) = mutex.try_lock() {
            return guard;
        }
        let mut rng = rand::thread_rng();
        let sleep_time: u64 = rng.gen_range(5..10);
        thread::sleep(Duration::from_millis(sleep_time));
        count += 1;

        if count > 500{
            println!("Acquire lock failed on mutex containig value {:?}", std::any::type_name::<T>());           
        }

    }
}

// Function to compute the measure for the Kalman filter from the raw GPS data
fn get_measure_forom_gps(
    measure: &Gps, reference: &Gps,
) -> Measure {
    let position = Point3::new(
        (measure.lat*f32::powf(10.0, -7.0) - reference.lat*f32::powf(10.0, -7.0)) * EARTH_CIRCUMFERENCE_METERS / 360.0,
        (measure.lon*f32::powf(10.0, -7.0) - reference.lon*f32::powf(10.0, -7.0)) * EARTH_CIRCUMFERENCE_METERS * LAT_FACTOR / 360.0,
        measure.h_msl*f32::powf(10.0, -3.0) - reference.h_msl*f32::powf(10.0, -3.0),
    );
    let velocity_xy = Vector2::new(measure.vel_n*f32::powf(10.0, -3.0), measure.vel_e*f32::powf(10.0, -3.0));
    let velocity_z = -measure.vel_d*f32::powf(10.0, -3.0);
    let mut pos_variance = Matrix3::zeros();
    pos_variance[(0, 0)] = 0.25 * (measure.h_acc*f32::powf(10.0, -3.0)).powf(2.0);
    pos_variance[(1, 1)] = 0.25 * (measure.h_acc*f32::powf(10.0, -3.0)).powf(2.0);
    pos_variance[(2, 2)] = 0.25 * (measure.v_acc*f32::powf(10.0, -3.0)).powf(2.0);
    let vel_variance = Matrix2::identity() * 0.25 * (measure.s_acc*f32::powf(10.0, -3.0)).powf(2.0);
    let vel_z_variance = 0.25 * (measure.s_acc*f32::powf(10.0, -3.0)).powf(2.0);
    let measure = Measure {
        position,
        velocity_xy,
        velocity_z,
        pos_variance,
        vel_variance,
        vel_z_variance,
        new_measure: true,
    };
    measure
}


// Function that keeps on controll if the GPS fix is obtained
fn wait_for_fix_tipe(gps_ref_arc: &Arc<Mutex<Gps>>) {
    loop {
        let gps_ref_lock = acquire_lock(gps_ref_arc);
        if gps_ref_lock.fix_type == 3 {
            drop(gps_ref_lock);
            break;
        }
        drop(gps_ref_lock);
        thread::sleep(Duration::from_millis(1000));
    }
}

fn on_message_imu(message: Imu, input: &Arc<Mutex<Input>>) {
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
    drop(input_lock);
}

fn on_message_gps(message: Gps, gps_ref_arc: &Arc<Mutex<Gps>>, measure_arc: &Arc<Mutex<Measure>>) {
    
    let mut gps_ref_lock = acquire_lock(gps_ref_arc);
    let mut measure_lock = acquire_lock(measure_arc);
    println!("Message recieved: {:?}", message);

    if gps_ref_lock.fix_type != 3 {
        *gps_ref_lock = message;
        println!("GPS fix obtained: {:?}", gps_ref_lock);
    }
    let measure = get_measure_forom_gps(&message, &gps_ref_lock);
    *measure_lock = measure;
    println!("Measure obtained: {:?}", measure);
    drop(measure_lock);
    drop(gps_ref_lock);
    println!("Mutex Dropped");
}

// Kalman predict function on new input
fn filter_predict(kalman: &mut ESKF, input: &mut Input, filter_ts: Duration) {
    kalman.predict(input.acceleration, input.rotation, filter_ts);
    input.new_input = false;
}

// Kalman update function on new measure
fn filter_update(kalman: &mut ESKF, measure: &mut Measure) {
    kalman
    .observe_position_velocity2d(measure.position, measure.pos_variance, measure.velocity_xy, measure.vel_variance)
    .unwrap();
    kalman
    .observe_height(measure.velocity_z, measure.vel_z_variance)
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
        acceleration: Vector3::new(0.0, 0.0, 0.0),
        rotation: Vector3::new(0.0, 0.0, 0.0),
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
        position: Point3::new(0.0, 0.0, 0.0),
        velocity_xy: Vector2::new(0.0, 0.0),
        velocity_z: 0.0,
        pos_variance: Matrix3::zeros(),
        vel_variance: Matrix2::zeros(),
        vel_z_variance: 0.0,
        new_measure: false,
    };

    // Creating ESKF object
    let filter = eskf::Builder::new()
        .acceleration_variance(0.01) // FIXME
        .rotation_variance(0.01) // FIXME
        .build();

    // Defining Mutex for thread share
    let gps_ref_mutex = Arc::new(Mutex::new(gps_ref));
    let measure_mutex = Arc::new(Mutex::new(measure));
    let input_mutex = Arc::new(Mutex::new(input));
    let filter_mutex = Arc::new(Mutex::new(filter));

    // DEBUG
    let debug = Debug {
        mqtt_recieve_process_counter:0,
        mqtt_send_process_counter:0,
        filter_process_counter:0,
    };

    let debug_mutex = Arc::new(Mutex::new(debug));


    // TODO: Add username and password authentication
    let mqqt_opts = MqttOptions::new("sailtrack-aggregator", "localhost", 1883);

    let (client, mut connection) = Client::new(mqqt_opts, 10);
    client.subscribe("sensor/gps0", QoS::AtMostOnce).unwrap();
    client.subscribe("sensor/imu0", QoS::AtMostOnce).unwrap();

    // // MQTT Callbacks thread
    let gps_ref_clone = Arc::clone(&gps_ref_mutex);
    let measure_clone = Arc::clone(&measure_mutex);
    let input_clone = Arc::clone(&input_mutex);
    let debug_clone = Arc::clone(&debug_mutex);
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
            // Degug
            let mut debug_lock = acquire_lock(&debug_clone);
            debug_lock.mqtt_recieve_process_counter += 1;
            drop(debug_lock);
        }
    });

    // Kalman filter thread
    let gps_ref_clone = Arc::clone(&gps_ref_mutex);
    let measure_clone = Arc::clone(&measure_mutex);
    let input_clone = Arc::clone(&input_mutex);
    let filter_clone = Arc::clone(&filter_mutex);
    let debug_clone = Arc::clone(&debug_mutex);
    thread::spawn(move || loop {
        // Check if the GPS fix has been obtained
        wait_for_fix_tipe(&gps_ref_clone);
        let thread_start = Instant::now();
        let mut measure_lock = acquire_lock(&measure_clone);
        let mut input_lock = acquire_lock(&input_clone);
        let mut filter_lock = acquire_lock(&filter_clone);

        // Reliable GPS and IMU data: predict and update
        if measure_lock.new_measure && input_lock.new_input {
            println!("Predict and update");
            filter_predict(&mut filter_lock, &mut input_lock, filter_ts);
            filter_update(&mut filter_lock, &mut measure_lock);
            drop(input_lock);
            drop(measure_lock);
            drop(filter_lock);
        }
        // Only reliable GPS data: only update
        else if !input_lock.new_input && measure_lock.new_measure {
            println!("Update");
            filter_update(&mut filter_lock, &mut measure_lock);
            drop(measure_lock);
            drop(filter_lock);
        }
        // Only reliable or old IMU data: only predict
        else {
            println!("Predict");
            filter_predict(&mut filter_lock, &mut input_lock, filter_ts);
            drop(input_lock);
            drop(filter_lock);
        }
        let elapsed = thread_start.elapsed();
        if elapsed.as_millis() < filter_ts.as_millis() {
            thread::sleep(filter_ts - elapsed);
        }
        // Degug
        let mut debug_lock = acquire_lock(&debug_clone);
        debug_lock.filter_process_counter += 1;
        drop(debug_lock);
    });


    //MQTT publish thread
    let gps_ref_clone = Arc::clone(&gps_ref_mutex);
    let filter_clone = Arc::clone(&filter_mutex);
    let debug_clone = Arc::clone(&debug_mutex);
    thread::spawn(move || loop {
        // Check if the GPS fix has been obtained
        wait_for_fix_tipe(&gps_ref_clone);
        let filter_lock = acquire_lock(&filter_clone);
        let position = filter_lock.position;
        let velocity = filter_lock.velocity;
        let quat_orient = filter_lock.orientation;
        drop(filter_lock);

        let euler_orient = quat_orient.euler_angles();
        let roll = euler_orient.0;
        let pitch = euler_orient.1;
        let heading = euler_orient.2;

        let sog = velocity.norm() * MPS_TO_KNTS_MULTIPLIER;
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
        let lat =
        position.x * 360.0 / EARTH_CIRCUMFERENCE_METERS / LAT_FACTOR + gps_ref_lock.lat*f32::powf(10.0, -7.0);
        let lon: f32 = position.y * 360.0 / EARTH_CIRCUMFERENCE_METERS + gps_ref_lock.lon*f32::powf(10.0, -7.0);
        let altitude = position.z + gps_ref_lock.h_msl*f32::powf(10.0, -3.0);
        drop(gps_ref_lock);

        let message = Boat {
            lon,
            lat,
            cog,
            sog,
            altitude,
            ascension_speed: velocity.z,
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

        // Degug
        let mut debug_lock = acquire_lock(&debug_clone);
        debug_lock.mqtt_send_process_counter += 1;
        drop(debug_lock);
    });

    let debug_clone = Arc::clone(&debug_mutex);
    let gps_ref_clone = Arc::clone(&gps_ref_mutex);
    loop {
        let debug_lock = acquire_lock(&debug_clone);
        println!(
            "MQTT send process counter: {}, MQTT receive process counter: {}, Filter process counter: {}",
            debug_lock.mqtt_send_process_counter, debug_lock.mqtt_recieve_process_counter, debug_lock.filter_process_counter
        );
        drop(debug_lock);
        thread::sleep(Duration::from_secs(1));
        let gps_ref_lock = acquire_lock(&gps_ref_clone);
        drop(gps_ref_lock);
        println!("GPS ref access granted:");
    };

}

