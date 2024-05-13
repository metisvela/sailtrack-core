use eskf::ESKF;
use std::thread;
use std::time::Duration;

use rumqttc::{Client, Event, Incoming, MqttOptions, QoS};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Euler {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Imu {
    euler: Euler,
    // TODO: Add fields
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Gps {
    fix_type: i32,
    // TODO: Add fields
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Boat {
    sog: f32,
    // TODO: Add fields
}

fn on_message_imu(filter: ESKF, message: Imu) {
    println!("Received IMU message: {message:?}");
    // TODO: Update filter
}

fn on_message_gps(filter: ESKF, message: Gps) {
    println!("Received GPS message: {message:?}");
    // TODO: Update filter
}

fn main() {
    // TODO: Add username and password authentication
    let mqqt_opts = MqttOptions::new("sailtrack-aggregator", "localhost", 1883);

    let mut filter = eskf::Builder::new()
        .acceleration_variance(0.01) // FIXME
        .rotation_variance(0.01) // FIXME
        .build();

    let (client, mut connection) = Client::new(mqqt_opts, 10);
    client.subscribe("sensor/gps0", QoS::AtMostOnce).unwrap();
    client.subscribe("sensor/imu0", QoS::AtMostOnce).unwrap();

    thread::spawn(move || {
        loop {
            let message = Boat {
                sog: 10.0, // FIXME
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
            thread::sleep(Duration::from_millis(200));
        }
    });

    for notification in connection.iter().flatten() {
        if let Event::Incoming(Incoming::Publish(packet)) = notification {
            match packet.topic.as_str() {
                "sensor/imu0" => on_message_imu(
                    filter,
                    serde_json::from_slice(packet.payload.as_ref()).unwrap(),
                ),
                "sensor/gps0" => on_message_gps(
                    filter,
                    serde_json::from_slice(packet.payload.as_ref()).unwrap(),
                ),
                _ => (),
            }
        }
    }
}
