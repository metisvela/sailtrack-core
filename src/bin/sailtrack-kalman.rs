use eskf::ESKF;
use log::{debug, info};
use map_3d::{geodetic2ned, ned2geodetic, Ellipsoid};
use nalgebra::{Matrix3, Point3, Rotation3, Vector3};
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

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "camelCase")]
struct Euler {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug, Default, Clone, Copy)]
struct Orientation {
    roll: f32,
    pitch: f32,
    heading: f32,
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

impl Default for Imu {
    fn default() -> Imu {
        Imu {
            euler: Euler {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            linear_accel: LinearAccel {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        }
    }
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
#[derive(Debug, Clone, Copy)]
struct BoatInfo {
    filter: ESKF,
    ref_pos: Gps,
    orientation: Orientation,
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

    // Initialize connection
    let mqttoptions = MqttOptions::new("eskf-demo", "localhost", 1883);
    let (client, mut connection) = Client::new(mqttoptions, 10);
    client.subscribe("sensor/imu0", QoS::AtMostOnce).unwrap();
    client.subscribe("sensor/gps0", QoS::AtMostOnce).unwrap();

    // Initialize filter
    let filter = eskf::Builder::new();
    filter.acceleration_variance(0.001);
    let boat_info = BoatInfo {
        filter: filter.build(),
        ref_pos: Gps::default(),
        orientation: Orientation::default(),
    };
    let boat_info_arc = Arc::new(RwLock::new(boat_info));

    // Spawn prediction thread
    let boat_info_mutex = boat_info_arc.clone();
    spawn(move || loop {
        // Get Boat Info
        let boat_info = boat_info_mutex.read().unwrap();
        let gps_ref = boat_info.ref_pos;
        let filter = boat_info.filter;
        let orientation = boat_info.orientation;
        drop(boat_info);

        // Get Boat Metrics
        let position = filter.position;
        let velocity = filter.velocity;

        // From IMU to Geodetic reference frame transformation
        let rotation_mtx =
            Rotation3::from_euler_angles(0.0, 0.0, -orientation.heading.to_radians());
        let neu_pos = rotation_mtx.transform_point(&position);

        let n: f64 = neu_pos.x as f64;
        let e: f64 = neu_pos.y as f64;
        let d: f64 = -neu_pos.z as f64;

        let lat0 = (gps_ref.lat * f32::powf(10.0, -7.0)).to_radians() as f64;
        let lon0 = (gps_ref.lon * f32::powf(10.0, -7.0)).to_radians() as f64;
        let alt0 = (gps_ref.h_msl * f32::powf(10.0, -3.0)) as f64;

        let (lat, lon, altitude) = ned2geodetic(n, e, d, lat0, lon0, alt0, Ellipsoid::default());

        let sog = (velocity.x.powi(2) + velocity.y.powi(2)).sqrt() * MPS_TO_KNTS_MULTIPLIER;
        let mut cog = f32::atan2(velocity.y, velocity.x).to_degrees();
        let mut drift = -1.0;
        if sog > 1.0 {
            cog = f32::atan2(velocity.y, velocity.x).to_degrees();
            cog = angle_unwrap(cog);
            let cog_180 = angle_wrap_180(cog);
            let head_180 = angle_wrap_180(orientation.heading);
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
            lon: lon.to_degrees() as f32,
            lat: lat.to_degrees() as f32,
            cog,
            sog,
            altitude: altitude as f32,
            ascension_speed: velocity.z,
            heading: orientation.heading,
            pitch: orientation.pitch,
            roll: orientation.roll,
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
    let mut delta = Instant::now();
    for event in connection.iter() {
        let event = event.unwrap();
        debug!("{event:?}");
        if let Incoming(Publish(message)) = event {
            let boat_info_mutex = boat_info_arc.clone();
            let boat_info = boat_info_mutex.read().unwrap();
            let mut filter = boat_info.filter;
            let mut ref_pos = boat_info.ref_pos;
            let mut orientation = boat_info.orientation;
            drop(boat_info);

            if message.topic == "sensor/imu0" {
                let input: Imu = serde_json::from_slice(&message.payload).unwrap();
                let acceleration = Vector3::new(
                    input.linear_accel.x,
                    input.linear_accel.y,
                    input.linear_accel.z,
                );
                orientation.roll = input.euler.x;
                orientation.pitch = -input.euler.y;
                orientation.heading = 360.0 - input.euler.z;
                let rotation = Vector3::new(input.euler.x, input.euler.y, input.euler.z);
                let elapsed = delta.elapsed();
                info!("Received IMU measurement: {input:?}. Updating filter prediction (delta={}ms)...", elapsed.as_millis());
                filter.predict(acceleration, rotation, elapsed);
                let mut boat_info = boat_info_mutex.write().unwrap();
                boat_info.filter = filter;
                boat_info.orientation = orientation;
                delta = Instant::now();
            } else if message.topic == "sensor/gps0" {
                let gps_data: Gps = serde_json::from_slice(&message.payload).unwrap();
                info!("Received GPS measurement: {gps_data:?}. Updating filter observation...");
                // If reference position is not set, set it and skip observation
                if ref_pos.fix_type != 3 {
                    ref_pos = gps_data;
                    let mut boat_info = boat_info_mutex.write().unwrap();
                    boat_info.ref_pos = ref_pos;
                    continue;
                }
                // Measure Unit Conversions
                let lat: f64 = (gps_data.lat * f32::powf(10.0, -7.0)).to_radians() as f64;
                let lon: f64 = (gps_data.lon * f32::powf(10.0, -7.0)).to_radians() as f64;
                let alt: f64 = (gps_data.h_msl * f32::powf(10.0, -3.0)) as f64;
                let vel_n = gps_data.vel_n * f32::powf(10.0, -3.0);
                let vel_e = gps_data.vel_e * f32::powf(10.0, -3.0);
                let vel_u = -gps_data.vel_d * f32::powf(10.0, -3.0);

                let lat0: f64 = (ref_pos.lat * f32::powf(10.0, -7.0)).to_radians() as f64;
                let lon0: f64 = (ref_pos.lon * f32::powf(10.0, -7.0)).to_radians() as f64;
                let alt0: f64 = (ref_pos.h_msl * f32::powf(10.0, -3.0)) as f64;

                let h_acc = gps_data.h_acc * f32::powf(10.0, -3.0);
                let v_acc = gps_data.v_acc * f32::powf(10.0, -3.0);
                let s_acc = gps_data.s_acc * f32::powf(10.0, -3.0);

                // GPS Data To Measure Conversions
                let (n, e, d) = geodetic2ned(lat, lon, alt, lat0, lon0, alt0, Ellipsoid::default());
                let position = Point3::new(n as f32, e as f32, -d as f32);
                let mut orizontal_std = 0.5 * h_acc / f32::sqrt(2.0);
                let mut vertical_std = 0.5 * v_acc;
                let mut speed_std = 0.5 * s_acc;
                if gps_data.fix_type != 3 {
                    orizontal_std *= 2.0;
                    vertical_std *= 2.0;
                    speed_std *= 2.0;
                }
                let pos_var = Matrix3::from_diagonal(&Vector3::new(
                    orizontal_std.powi(2),
                    orizontal_std.powi(2),
                    vertical_std.powi(2),
                ));

                let velocity = Vector3::new(vel_n, vel_e, vel_u);
                let vel_variance = speed_std.powi(2) * Matrix3::identity();

                // Rotation to IMU Reference Frame
                let rotation =
                    Rotation3::from_euler_angles(0.0, 0.0, orientation.heading.to_radians());

                let rot_position = rotation.transform_point(&position);
                let rot_pos_var = rotation * pos_var * rotation.transpose();

                let rot_velocity = rotation.transform_vector(&velocity);
                let rot_vel_variance = rotation * vel_variance * rotation.transpose();
                filter.observe_position(rot_position, rot_pos_var).unwrap();

                filter
                    .observe_velocity(rot_velocity, rot_vel_variance)
                    .unwrap();

                let mut boat_info = boat_info_mutex.write().unwrap();
                boat_info.filter = filter;
            }
        }
    }
}
