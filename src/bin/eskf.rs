use nalgebra::{Matrix2, Matrix3, Point3, Vector2, Vector3};
use paho_mqtt::Client;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Measure realization
    let acc = Vector3::new(1.1, 2.1, 0.1);
    let orient = Vector3::new(0.4, 0.7, 0.3);
    let pos = Point3::new(10.0, 15.0, 0.0);
    let vel = Vector2::new(5.0, 3.0);
    let high: f32 = 12.2;
    let high_var: f32 = 0.01;

    // Measure variances
    let acc_var: f32 = 0.01;
    let orient_var: f32 = 0.01;
    let pos_var: f32 = 0.02;
    let vel_var: f32 = 0.001;
    let ts = std::time::Duration::from_millis(10);

    // Measure struct
    struct Imu {
        acceleration: Vector3<f32>,
        rotation: Vector3<f32>,
    }

    struct Gnss {
        position: Point3<f32>,
        pos_variance: Matrix3<f32>,
        velocity: Vector2<f32>,
        vel_variance: Matrix2<f32>,
        high: f32,
        high_var: f32,
    }

    let imu_read = Imu {
        acceleration: acc,
        rotation: orient,
    };

    let gps = Gnss {
        position: pos,
        pos_variance: Matrix3::identity() * pos_var,
        velocity: vel,
        vel_variance: Matrix2::identity() * vel_var,
        high,
        high_var,
    };

    println!("GNSS and IMU INIT");

    // Just making sure that we didn't accidentally create a null pointer here
    println!(
        "IMU acceleration x: {}, y: {}, z: {}",
        imu_read.acceleration.x, imu_read.acceleration.y, imu_read.acceleration.z
    );
    println!(
        "IMU rotation x: {}, y: {}, z: {}",
        imu_read.rotation.x, imu_read.rotation.y, imu_read.rotation.z
    );
    println!(
        "GNSS position x: {}, y: {}, z: {}",
        gps.position.x, gps.position.y, gps.position.z
    );
    println!("GNSS velocity x: {}, y: {}", gps.velocity.x, gps.velocity.y);
    print!("GNSS high: {}", gps.high);
    // ESKF Object construction
    let mut filter = eskf::Builder::new()
        .acceleration_variance(acc_var)
        .rotation_variance(orient_var)
        .build();

    println!("\n Filter built");

    filter.predict(imu_read.acceleration, imu_read.rotation, ts);

    println!("Filter prediction");

    let _result = filter.observe_position_velocity2d(
        gps.position,
        gps.pos_variance,
        gps.velocity,
        gps.vel_variance,
    );

    println!("Filter filtered prediction with XY velocity");

    let _result_high = filter.observe_height(gps.high, gps.high_var);

    println!("Filter filtered prediction with height");

    Ok(())
}
