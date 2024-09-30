use eskf::ESKF;
use log::{debug, info};
use nalgebra::{Point3, Vector3};
use rumqttc::Event::Incoming;
use rumqttc::Packet::Publish;
use rumqttc::{Client, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

// FIXME: Use correct message format
#[derive(Deserialize, Debug)]
struct ImuMeasurement {
    acceleration: Vector3<f32>,
    rotation: Vector3<f32>,
}

// FIXME: Use correct message format
#[derive(Deserialize, Debug)]
struct GpsMeasurement {
    position: Point3<f32>,
    variance: f32,
}

// FIXME: Use correct message format
#[derive(Serialize)]
struct BoatPrediction {
    position: Point3<f32>,
    velocity: Vector3<f32>,
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
    let filter_arc = Arc::new(RwLock::new(eskf::Builder::new().build()));

    // Spawn prediction thread
    let filter_mutex = filter_arc.clone();
    spawn(move || loop {
        let filter = filter_mutex.read().unwrap();
        let prediction = BoatPrediction {
            position: filter.position,
            velocity: filter.velocity,
        };
        drop(filter);
        client
            .publish(
                "boat",
                QoS::AtLeastOnce,
                false,
                serde_json::to_vec(&prediction).unwrap(),
            )
            .unwrap();
        sleep(Duration::from_millis(1000));
    });

    // Process MQTT events
    let mut delta = Instant::now();
    for event in connection.iter() {
        let event = event.unwrap();
        debug!("{event:?}");
        if let Incoming(Publish(message)) = event {
            let filter_mutex = filter_arc.clone();
            let mut filter = filter_mutex.write().unwrap();
            if message.topic == "sensor/imu0" {
                // FIXME: Correctly extract acceleration and rotation from the IMU measurement
                let measurement: ImuMeasurement = serde_json::from_slice(&message.payload).unwrap();
                let elapsed = delta.elapsed();
                info!("Received IMU measurement: {measurement:?}. Updating filter prediction (delta={}ms)...", elapsed.as_millis());
                filter.predict(
                    measurement.acceleration,
                    measurement.rotation,
                    elapsed,
                );
                delta = Instant::now();
            } else if message.topic == "sensor/gps0" {
                // FIXME: Correctly extract position and variance from the GPS measurement
                let measurement: GpsMeasurement = serde_json::from_slice(&message.payload).unwrap();
                info!("Received GPS measurement: {measurement:?}. Updating filter observation...");
                filter
                    .observe_position(
                        measurement.position,
                        ESKF::variance_from_element(measurement.variance),
                    )
                    .unwrap();
            }
        }
    }
}