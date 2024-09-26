use kalmanfilt::kalman::kalman_filter::KalmanFilter as Kalman;
use log::{debug, info};
use env_logger;
use rand::Rng;
use nalgebra::{OMatrix, OVector, U3, U6};
use rumqttc::Event::Incoming;
use rumqttc::Packet::Publish;
use rumqttc::{Client, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

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

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
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

impl Default for Gps {
    fn default() -> Gps {
        Gps {
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
        }
    }
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

impl Default for Measure {
    fn default() -> Measure {
        Measure {
            meas: OVector::<f32, U6>::zeros(),
            meas_variance: OMatrix::<f32, U6, U6>::identity(),
            variance_handler: MeasureCollection::<OVector<f32, U6>>::new(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Input {
    acceleration: OVector<f32, U3>,
    orientation: OVector<f32, U3>,
}

impl Default for Input {
    fn default() -> Input {
        Input {
            acceleration: OVector::<f32, U3>::zeros(),
            orientation: OVector::<f32, U3>::zeros(),
        }
    }
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

fn read_arc<T>(arc: &Arc<RwLock<T>>, line: u32) -> T
where
    T: Clone,
{
    let mut iter = 0;
    let var: T;    
    loop{
        match arc.try_read() {
            Ok(content) => {
                var = content.clone();
                break;
            },
            Err(_) => {
                iter += 1;
                if iter > 100 {
                    println!(
                        "Failed to read mutex {:?} at line {}",
                        std::any::type_name::<T>(),
                        line
                    );
                }
                let mut rng = rand::thread_rng();
                let sleep_time = rng.gen_range(5..10);
                sleep(Duration::from_millis(sleep_time));
            }
        }
    }
    return var;
}

fn write_arc<T>(arc: &Arc<RwLock<T>>, value: T, line: u32)
where
    T: Clone,
{
    let mut iter = 0;
    loop{
        match arc.try_write() {
            Ok(mut content) => {
                *content = value.clone();
            }
            Err(_) => {
                iter += 1;
                if iter > 100 {
                    println!(
                        "Failed to write mutex {:?} at line {}",
                        std::any::type_name::<T>(),
                        line
                    );
                }
                let mut rng = rand::thread_rng();
                let sleep_time = rng.gen_range(5..10);
                sleep(Duration::from_millis(sleep_time));
            }
        }
    }
}


fn on_message_imu(message: Imu) -> Input {
    let accel_vec = vec![
        message.linear_accel.x,
        message.linear_accel.y,
        message.linear_accel.z,
    ];
    let acceleration = OVector::<f32, U3>::from_iterator(accel_vec);
    let orient_vec = vec![message.euler.x, -message.euler.y, 360.0 - message.euler.z];
    let orientation = OVector::<f32, U3>::from_iterator(orient_vec);
    Input {
        acceleration,
        orientation,
    }
}

fn on_message_gps(
    message: Gps,
    gps_ref_arc: &Arc<RwLock<Gps>>,
    measure: &mut Measure,
) {
    let gps_ref = read_arc(gps_ref_arc, line!());
    if gps_ref.fix_type != 3 {
        write_arc(gps_ref_arc, message, line!());
    }
    let meas_vec = vec![
        (message.lat * f32::powf(10.0, -7.0) - gps_ref.lat * f32::powf(10.0, -7.0))
            * EARTH_CIRCUMFERENCE_METERS
            / 360.0,
        (message.lon * f32::powf(10.0, -7.0) - gps_ref.lon * f32::powf(10.0, -7.0))
            * EARTH_CIRCUMFERENCE_METERS
            * LAT_FACTOR
            / 360.0,
        message.h_msl * f32::powf(10.0, -3.0) - gps_ref.h_msl * f32::powf(10.0, -3.0),
        message.vel_n * f32::powf(10.0, -3.0),
        message.vel_e * f32::powf(10.0, -3.0),
        -message.vel_d * f32::powf(10.0, -3.0),
    ];
    let meas: OVector<f32, U6> = OVector::<f32, U6>::from_iterator(meas_vec);
    let accuracy_penality_factor = 100.0;
    measure.meas = meas;
    measure.variance_handler.add(meas);
    measure.meas_variance = measure.variance_handler.get_variance();
    if message.fix_type != 3 {
        measure.meas_variance *= accuracy_penality_factor;
    }
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
    // Initialize logger
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_target(false)
        .init();

    // Defining structures and filter parameters
    let filter_ts = Duration::from_millis(KALMAN_SAMPLE_TIME_MS);

    let gps_ref = Gps::default();
    let input = Input::default();

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

    let mut filter = Kalman::<f32, nalgebra::Const<6>, nalgebra::Const<6>, nalgebra::Const<3>> {
        x: OVector::<f32, U6>::zeros(),
        P: OMatrix::<f32, U6, U6>::identity(),
        F: transition_mtx,
        H: output_mtx,
        B: Some(input_mtx),
        Q: noise_state_cov,
        R: noise_meas_cov,
        ..Default::default()
    };

    // Initialize connection
    // TODO: Add username and password authentication
    let mqqt_opts = MqttOptions::new("sailtrack-kalman", "localhost", 1883);
    // mqqt_opts.set_credentials("mosquitto", "sailtrack");
    let (client, mut connection) = Client::new(mqqt_opts, 10);
    client.subscribe("sensor/imu0", QoS::AtMostOnce).unwrap();
    client.subscribe("sensor/gps0", QoS::AtMostOnce).unwrap();

    // Initialize filter
    let filter_arc = Arc::new(RwLock::new(filter));
    let gps_ref_arc = Arc::new(RwLock::new(gps_ref));
    let input_arc = Arc::new(RwLock::new(input));

    // Spawn Sender thread
    let gps_ref_mutex = gps_ref_arc.clone();
    let input_mutex = input_arc.clone();
    let filter_mutex = filter_arc.clone();
    spawn(move || loop {
        let input = read_arc(&input_mutex, line!());
        let roll = input.orientation.x;
        let pitch = input.orientation.y;
        let heading = input.orientation.z;

        let filter = read_arc(&filter_mutex, line!());
        let position = filter.x.fixed_rows::<3>(0);
        let velocity = filter.x.fixed_rows::<3>(3);
        
        // Position metrics
        let gps_ref = read_arc(&gps_ref_mutex, line!());
        let lat = position.x * 360.0 / EARTH_CIRCUMFERENCE_METERS / LAT_FACTOR
            + gps_ref.lat * f32::powf(10.0, -7.0);
        let lon: f32 =
            position.y * 360.0 / EARTH_CIRCUMFERENCE_METERS + gps_ref.lon * f32::powf(10.0, -7.0);
        let altitude = position.z + gps_ref.h_msl * f32::powf(10.0, -3.0);
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

        sleep(Duration::from_millis(1000 / MQTT_PUBLISH_FREQ_HZ));
    });

    // Process MQTT events
    let filter_mutex = filter_arc.clone();
    let gps_ref_arc = gps_ref_arc.clone();
    let mut measure = Measure::default();
    let mut delta = Instant::now();
    for event in connection.iter() {
        let event = event.unwrap();
        debug!("{event:?}");
        if let Incoming(Publish(message)) = event {
            if message.topic == "sensor/imu0" {
                let payload = message.payload.clone();
                let input = on_message_imu(serde_json::from_slice(payload.as_ref()).unwrap(),);
                let elapsed = delta.elapsed();
                info!("Received IMU measurement: {input:?}. Updating filter prediction (delta={}ms)...", elapsed.as_millis());
                filter_predict(&mut filter, &input);
                write_arc(&filter_mutex, filter, line!());
                delta = Instant::now();
            } else if message.topic == "sensor/gps0" {
                // FIXME: Correctly extract position and variance from the GPS measurement
                let gps_measure: Gps = serde_json::from_slice(&message.payload).unwrap();
                on_message_gps(gps_measure, &gps_ref_arc, &mut measure);
                info!("Received GPS measurement: {measure:?}. Updating filter observation...");
                filter_update(&mut filter, &measure).unwrap();
            }
        }
    }
}
