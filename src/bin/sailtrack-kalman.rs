use kalmanfilt::kalman::kalman_filter::KalmanFilter as Kalman;
use rand::Rng;
use std::sync::{Arc, RwLock};
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
enum SyncEvent {
    GpsReceived,
    ImuReceived,
}

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
#[derive(Debug, Clone)]
struct MeasureCollection<OVector> {
    buffer: Vec<OVector>,
    capacity: usize,
    index: usize,
}

#[derive(Debug, Clone)]
struct Measure {
    meas: OVector<f32, U6>,
    meas_variance: OMatrix<f32, U6, U6>,
    variance_handler: MeasureCollection<OVector<f32, U6>>,
}

#[derive(Debug, Clone, Copy)]
struct Input {
    acceleration: OVector<f32, U3>,
    orientation: OVector<f32, U3>,
}

impl MeasureCollection<OVector<f32, U6>> {
    fn new() -> Self {
        let capacity: usize = 5;
        let index: usize = 0;
        MeasureCollection {
            buffer: Vec::<OVector<f32, U6>>::with_capacity(5),
            capacity,
            index,
        }
    }

    fn add(&mut self, value: OVector<f32, U6>) {
        if self.index > self.capacity - 1 {
            self.index = 0;
        }
        self.buffer.insert(self.index, value);
        self.index += 1;
    }

    fn get_variance(&self) -> OMatrix<f32, U6, U6> {
        let mut covariance = OMatrix::<f32, U6, U6>::zeros();
        if self.buffer.len() <= self.capacity {
            covariance = OMatrix::<f32, U6, U6>::identity()
        }
        let mut sum = OVector::<f32, U6>::zeros();
        for observation in &self.buffer {
            sum += observation;
        }
        let mean = sum / self.capacity as f32;
        for observation in &self.buffer {
            let centered_observation = observation - mean;
            covariance += centered_observation * centered_observation.transpose();
        }
        covariance /= (self.capacity - 1) as f32;
        covariance
    }
}

fn read_arc<T>(arc: &Arc<RwLock<T>>, line: u32) -> std::sync::RwLockReadGuard<T> {
    let mut iter = 1;
    loop {
        match arc.read() {
            Ok(content) => {
                return content;
            }
            Err(_) => {
                let mut rng = rand::thread_rng();
                let sleep_time: u64 = rng.gen_range(5..10);
                iter += 1;
                if iter > 100 {
                    println!(
                        "Failed to acquire lock on mutex lock of class {:?} at line {}",
                        std::any::type_name::<T>(),
                        line
                    );
                }
                thread::sleep(Duration::from_millis(sleep_time));
            }
        }
    }
}

fn write_arc<T>(arc: &Arc<RwLock<T>>, line: u32) -> std::sync::RwLockWriteGuard<T> {
    let mut iter = 1;
    loop {
        match arc.write() {
            Ok(content) => {
                return content;
            }
            Err(_) => {
                let mut rng = rand::thread_rng();
                let sleep_time: u64 = rng.gen_range(5..10);
                iter += 1;
                if iter > 100 {
                    println!(
                        "Failed to acquire lock on mutex lock of class {:?} at line {}",
                        std::any::type_name::<T>(),
                        line
                    );
                }
                thread::sleep(Duration::from_millis(sleep_time));
            }
        }
    }
}
// Function to compute the measure for the Kalman filter from the raw GPS data
fn get_measure_forom_gps(gps_data: &Gps, reference: &Gps, measure_struct: &mut Measure) {
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
        -gps_data.vel_d * f32::powf(10.0, -3.0),
    ];
    let meas: OVector<f32, U6> = OVector::<f32, U6>::from_iterator(meas_vec);
    let accuracy_penality_factor = 100.0;
    measure_struct.meas = meas;
    measure_struct.variance_handler.add(meas);
    measure_struct.meas_variance = measure_struct.variance_handler.get_variance();
    if gps_data.fix_type != 3 {
        measure_struct.meas_variance *= accuracy_penality_factor;
    }
}

fn on_message_imu(message: Imu, input: &Arc<RwLock<Input>>) {
    let accel_vec = vec![
        message.linear_accel.x,
        message.linear_accel.y,
        message.linear_accel.z,
    ];
    let accel = OVector::<f32, U3>::from_iterator(accel_vec);
    let orient_vec = vec![message.euler.x, -message.euler.y, 360.0 - message.euler.z];
    let orient = OVector::<f32, U3>::from_iterator(orient_vec);
    let mut input_lock = write_arc(input, line!());
    input_lock.acceleration = accel;
    input_lock.orientation = orient;
    drop(input_lock);
}

fn on_message_gps(
    message: Gps,
    gps_ref_arc: &Arc<RwLock<Gps>>,
    measure_arc: &Arc<RwLock<Measure>>,
) {
    let mut gps_ref_lock = write_arc(gps_ref_arc, line!());
    let mut measure_lock = write_arc(measure_arc, line!());

    if gps_ref_lock.fix_type != 3 {
        *gps_ref_lock = message;
    }
    get_measure_forom_gps(&message, &gps_ref_lock, &mut measure_lock);
    drop(measure_lock);
    drop(gps_ref_lock);
}

// Kalman predict function on new input
fn filter_predict(kalman: &mut Kalman<f32, U6, U6, U3>, input: &Input) {
    kalman.predict(Some(&input.acceleration), None, None, None);
}

// Kalman update function on new measure
fn filter_update(
    kalman: &mut Kalman<f32, U6, U6, U3>,
    measure: &Measure,
) -> Result<(), &'static str> {
    match kalman.update(&measure.meas, Some(&measure.meas_variance), None) {
        Ok(_) => Ok(()),
        Err(_) => {
            println!(
                "measure: {:?}, variance: {:?}",
                measure.meas, measure.meas_variance
            );
            Err("Error occurred in filter update function")
        }
    }
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
        variance_handler: MeasureCollection::<OVector<f32, U6>>::new(),
    };

    // Creating ESKF object
    let w_std = 0.001;
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
        x: OVector::<f32, U6>::zeros(),
        P: OMatrix::<f32, U6, U6>::identity(),
        F: transition_mtx,
        H: output_mtx,
        B: Some(input_mtx),
        Q: noise_state_cov,
        R: noise_meas_cov,
        ..Default::default()
    };
    // Defining Event Channels
    let (tx, rx) = crossbeam_channel::unbounded();

    // Defining Mutex for thread share
    let gps_ref_mutex = Arc::new(RwLock::new(gps_ref));
    let measure_mutex = Arc::new(RwLock::new(measure));
    let input_mutex = Arc::new(RwLock::new(input));
    let filter_mutex = Arc::new(RwLock::new(filter));

    // TODO: Add username and password authentication
    let mqqt_opts = MqttOptions::new("sailtrack-kalman", "localhost", 1883);
    //mqqt_opts.set_credentials("mosquitto", "sailtrack");

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
                        match tx.try_send(SyncEvent::ImuReceived) {
                            Ok(_) => (),
                            Err(_) => continue,
                        }
                    }
                    "sensor/gps0" => {
                        let payload = packet.payload.clone(); // Clone the payload for later use
                        on_message_gps(
                            serde_json::from_slice(payload.as_ref()).unwrap(),
                            &gps_ref_clone,
                            &measure_clone,
                        );
                        match tx.try_send(SyncEvent::GpsReceived) {
                            Ok(_) => (),
                            Err(_) => continue,
                        }
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
        while gps_ref_clone.read().unwrap().fix_type != 3 {
            thread::sleep(Duration::from_millis(500));
        }

        let thread_start = Instant::now();
        let mut gps_recieved_flag = false;
        let mut imu_recieved_flag = true;
        let measure = measure_clone.read().unwrap();
        let zero_input: Input = Input {
            acceleration: OVector::<f32, U3>::zeros(),
            orientation: input.orientation,
        };
        let input = input_clone.read().unwrap();

        for _message in rx.try_iter() {
            match rx.try_recv() {
                Ok(SyncEvent::GpsReceived) => gps_recieved_flag = true,
                Ok(SyncEvent::ImuReceived) => imu_recieved_flag = true,
                Err(_) => (),
            }
        }

        let mut filter_lock = write_arc(&filter_clone, line!());
        match (gps_recieved_flag, imu_recieved_flag) {
            (true, true) => {
                filter_predict(&mut filter_lock, &input);
                filter_update(&mut filter_lock, &measure).unwrap();
                drop(filter_lock);
            }
            (true, false) => {
                filter_update(&mut filter_lock, &measure).unwrap();
                drop(filter_lock);
            }
            (false, true) => {
                filter_predict(&mut filter_lock, &input);
                drop(filter_lock);
            }
            (false, false) => {
                filter_predict(&mut filter_lock, &zero_input);
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
        let input_lock = read_arc(&input_clone, line!());
        let roll = input_lock.orientation.x;
        let pitch = input_lock.orientation.y;
        let heading = input_lock.orientation.z;
        drop(input_lock);

        let filter_read = {
            let filter_lock = read_arc(&filter_clone, line!());
            filter_lock.clone()
        };

        let position = filter_read.x.fixed_rows::<3>(0);
        let velocity = filter_read.x.fixed_rows::<3>(3);
        // Position metrics
        let gps_ref_lock = read_arc(&gps_ref_clone, line!());
        let lat = position.x * 360.0 / EARTH_CIRCUMFERENCE_METERS / LAT_FACTOR
            + gps_ref_lock.lat * f32::powf(10.0, -7.0);
        let lon: f32 = position.y * 360.0 / EARTH_CIRCUMFERENCE_METERS
            + gps_ref_lock.lon * f32::powf(10.0, -7.0);
        let altitude = position.z + gps_ref_lock.h_msl * f32::powf(10.0, -3.0);
        drop(gps_ref_lock);
        let z_speed = velocity.z * MPS_TO_KNTS_MULTIPLIER;

        // Velocity metrics
        let sog = (velocity.x.powi(2) + velocity.y.powi(2)).sqrt() * MPS_TO_KNTS_MULTIPLIER;
        let mut cog = heading;

        let mut drift = -1.0;
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

        // Publish boat metrics
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
                "boat",
                QoS::AtLeastOnce,
                false,
                serde_json::to_vec(&message).unwrap(),
            )
            .unwrap();

        thread::sleep(Duration::from_millis(1000 / MQTT_PUBLISH_FREQ_HZ));
    }
}
