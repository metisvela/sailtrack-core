# Developer's Guide

This guide is intended to introduce new developers to all the aspects needed to contribute to SailTrack Core.
If you haven't already, read the [project repository](https://github.com/metisvela/sailtrack) carefully to familiarize yourself with the SailTrack project and its components.
In this guide, we will assume that you're already familiar with the overall behavior of the system.

## Overview

SailTrack Core is based on [DietPi](https://dietpi.com), a modified version of [Raspberry Pi OS](https://www.raspberrypi.com/software/), which is in turn a modified version of [Debian](https://www.debian.org).
When the OS boots up, the following tasks are performed continuously until shutdown:
* Serving the Wi-Fi access point (i.e. the "SailTrack-CoreNet" Wi-Fi network)
* Running the MQTT broker (i.e. [Mosquitto](https://mosquitto.org))
* Running the database (i.e. [InfluxDB](https://www.influxdata.com))
* Running the metrics collector (i.e. [Telegraf](https://www.influxdata.com/time-series-platform/telegraf/), which takes the metrics coming from the sensors via MQTT, sends them to Grafana for live visualization, and sends them to InfluxDB for storing)
* Running the visualization tool (i.e. [Grafana](https://grafana.com))
* Running the SailTrack scripts (more info in the [section below](#sailtrack-scripts))

## Development

The idea behind the development of the SailTrack Core software is to have in this repository everything that is needed to automatically set up the Raspberry Pi from a clean state.
This is done to avoid any non-reproducible and non-trackable manual work on the Raspberry Pi.
In this way, if for any reason there's a need to erase all the data and restore the Raspberry Pi to a stable state, this can be done easily.
Therefore, any change to any software component in the Raspberry Pi system should be present in this repository.
Changes are submitted via [Pull Requests](https://github.com/metisvela/sailtrack-core/pulls), following the [GitHub Flow](https://docs.github.com/en/get-started/using-github/github-flow).

The following are the common guidelines when adding new files to the repository:

* Interpreted scripts (e.g. Python scripts, Bash scripts, etc.) should be added to the [`sailtrack`](sailtrack) directory.
* Compiled scripts (currently only Rust scripts are supported) should be added to the [`src/bin`](src/bin) directory.
* Static files (e.g. configuration files, images, dashboards, maps, etc.) should be added in a subdirectory of [`rootfs`](rootfs) according to the desired location of the file in the filesystem.

A very common example is the addition of a script with a correspondent service file.
According to the development workflow, this addition must not be done only directly in the running instance of SailTrack Core OS.
Instead, it must be applied in the repository too.
For this particular case, according to the guidelines, the script should go in `sailtrack` (if interpreted) or `src/bin` (if compiled), and the service file should go in `rootfs/etc/systemd/system`, because service files must be placed in `/etc/systemd/system` in the Raspberry Pi.

> [!WARNING]
> If both an interpreted and a compiled script exist with the same name, the compiled script will take precedence and will be included in the OS.

### Using Rust

Developing Rust scripts requires some more effort compared to other interpreted scripts, which can be just placed in the `sailtrack` folder.
Rust scripts must go through the compilation process to produce the binary that can be embedded in the OS.
Moreover, the source files must be cross-compiled to produce a binary that can be run on the Raspberry Pi.
This extra effort comes with the advantage of having more reliable and performant scripts that can handle heavier traffic and computations.

The following tools are needed to use Rust in SailTrack Core:

* [The Rust toolchain](https://www.rust-lang.org/tools/install)
* [Docker](https://docs.docker.com/engine/install) (required for cross-compilation)
* [Cross](https://github.com/cross-rs/cross#installation) (required for cross-compilation)
* [Cargo Make](https://github.com/sagiegurari/cargo-make?tab=readme-ov-file#installation) (required for deployment)

The development workflow for Rust follows the same workflow of any normal Rust crate, with the [`Cargo.toml`](Cargo.toml) manifest defining the project metadata and the binaries under [`src/bin`](src/bin).
On top of the normal Cargo commands that can be used when developing Rust (e.g. `build`, `test`, `fmt`, `clippy`, etc.), the default Cargo Make task can be used to perform an overall check of the project:
```bash
cargo make
```

This command will:

* Cross-compile the binaries in release mode for the Raspberry Pi
* Run `clippy` to lint the code
* Check the formatting

Additionally, the [Make](https://github.com/metisvela/sailtrack-core/actions/workflows/make.yml) GitHub Action runs `cargo make` on every push and pull request.

## Deployment

The deployment process has the goal of creating the SailTrack Core OS image that can be easily flashed onto the Raspberry Pi SD card.
To make the installation process as seamless as possible, the provided image contains an automated procedure that, after having installed the OS, sets up all the necessary components and scripts.
This is achieved in an automated way using [GitHub Actions](https://github.com/features/actions) and [DietPi-Automation](https://dietpi.com/docs/usage/#how-to-do-an-automatic-base-installation-at-first-boot-dietpi-automation).

The deployment process starts with the [Release](https://github.com/metisvela/sailtrack-core/actions/workflows/release.yml) GitHub Action, which gets triggered whenever a tag following the [Semantic Versioning](https://semver.org) format (e.g. `v1.2.3`) is pushed.
The GitHub Action starts by running the `cargo make package` task, which checks and compiles all the binaries and runs the [`sailtrack-package`](.make/sailtrack-package) script, which performs the following:

* Downloads the latest DietPi OS image
* Mounts the image (needed for copying files into it)
* Copies the contents of the repository to the appropriate locations, in particular:
  * All the files in the repository except the `rootfs` directory and unnecessary files and directories (like this one, the `hardware` directory, the `.gitignore`, etc.) get copied to the `/boot` folder, preserving the subdirectories structure.
  * All the files in the `rootfs` folder get copied to the root folder (`/`), preserving the subdirectories structure.
* Performs the final steps (renaming the image, including the hashes, compressing the file, etc.)

Once these first jobs have been completed, the GitHub Action proceeds to publish a [GitHub Release](https://github.com/metisvela/sailtrack-core/releases), including the generated image as an asset.

The built image will have the files in the `/boot` folder easily accessible for in-field modifications by plugging the SD card into a PC, and the files copied from the `rootfs` folder will automatically be placed in the required locations.

> [!NOTE]
> The SailTrack OS image can be built by manually running the [Package](https://github.com/metisvela/sailtrack-core/actions/workflows/package.yml) GitHub Action and downloading the produced artifact.

Once the OS image is flashed and running on the Raspberry Pi, the first-run setup will take place.
The first-run setup uses Dietpi-Automation, which is configured in the [`dietpi.txt`](dietpi.txt) and [`Automation_Custom_Script.sh`](Automation_Custom_Script.sh) files.

The `dietpi.txt` file contains all the settings for DietPi (such as the networking configuration, the time zone, etc.) and the automation configuration, such as the software to install and how to configure it.
Since DietPi-Automation's built-in features aren't enough to fully configure the OS, a bash script is needed, namely `Automation_Custom_Script.sh`.
This script runs after the DietPi configuration, and it finishes the steps needed to set up SailTrack Core (e.g. it installs the missing packages, it sets up authentication tokens, etc.).
Once the script ends, the Raspberry Pi is rebooted, and SailTrack Core is then successfully deployed.

### Upload scripts to a running instance

To update the `sailtrack-*` scripts on o running SailTrack Core instance, the `cargo make upload` task can be used while being connected to "SailTrack-CoreNet":

```bash
cargo make upload
```

This task will build and upload all the `sailtrack-*` scripts found locally.
By default, the upload process will target SailTrack Core's default IP, namely `192.168.42.1`.
However, an optional argument can be passed to specify a custom IP address:

```bash
cargo make upload <IP-ADDRESS>
```

Where `<IP-ADDRESS>` is the IP address of SailTrack Core in the local network.

> [!NOTE]
> The `upload` task only uploads the scripts.
> If the script is run by a service, the service needs to be restarted to actually run the new script.

## Components

In this section, each component of the system will be analyzed in detail.

### Wi-Fi

The Wi-Fi access point is served using the DietPi's built-in "Wi-Fi Hotspot" software package.
All it's needed is to enable the installation of the software package and configure the details of the WiFi network in the `dietpi.txt` file (under `AUTO_SETUP_INSTALL_SOFTWARE_ID`), and DietPi-Automation will take care of installing and setting it up.

### MQTT Broker (Mosquitto)

The MQTT broker is installed using the DietPi's built-in "Mosquitto" software package, which installs and sets up the Mosquitto broker.
As for the Wi-Fi, the installation is enabled in the `dietpi.txt` file.

### Time-Series Database (InfluxDB)

The time-series database is installed using the DietPi built-in "InfluxDB" software package, which installs and sets up the InfluxDB database.
Again here, all it's needed is to enable the software package in the `dietpi.txt` file.

### Metrics Collector (Telegraf)

Unlike the other components, the metrics collector is not included in the built-in software list of DietPi, which means that it has to be manually installed.
This is done in the `Automation_Custom_Script.sh` script, using a DietPi function called `G_AGI`, which calls the `apt install` command.

Additionally, Telegraf requires a configuration file in `etc/telegraf`.
As for the guidelines, the addition of a static configuration file is done by adding it in [`rootfs/etc/telegraf`](rootfs/etc/telegraf).

### Visualization Tool (Grafana)

Grafana is included in the DietPi software list.
Therefore, it's installed by adding the software package to the `dietpi.txt` file.

Additionally, Grafana requires some configuration files, assets for the dashboards, and some setup.
This is done by configuring Grafana in `Automation_Custom_Script.sh` and putting the configuration files and assets in `rootfs` accordingly.

### SailTrack Scripts

* [`sailtrack-x708_softsd`](sailtrack/sailtrack-x708_softsd) - Shell script that powers off the Raspberry Pi by sending an impulse to the [Geekworm X708](https://wiki.geekworm.com/X708) power HAT.
    This is required in order to perform a full software shutdown (software shutdown + battery power disconnected).
    Otherwise, you will only be able to perform a software shutdown.
* [`sailtrack-x708_pwr`](sailtrack/sailtrack-x708_pwr) - Shell script that performs a software shutdown or reboot depending on an impulse coming from the Geekworm X708 power HAT.
    This is needed to perform a full shutdown or reboot using a physical power button (the HAT sends the signal to this script, which performs the software shutdown/reboot, and then the HAT cuts the power).
* [`sailtrack-timesync`](sailtrack/sailtrack-timesync) - Python script that continuously listens to the `sensor/gps0` topic for a message that contains the `epoch` field, setting the system time accordingly to the received epoch. 
    This is needed because the Raspberry Pi has no RTC clock built-in and will therefore lose track of the time and date as soon as it is shut down.
    Instead, [SailTrack Radio](https://github.com/metisvela/sailtrack-radio) has a built-in RTC clock for the GPS, and therefore it sends, along with the GPS data, the current time and date.
* [`sailtrack-status`](sailtrack/sailtrack-status) - Python script that periodically sends the status data (e.g. battery voltage, CPU load, CPU temperature, etc.) of the module. 
    Needed for logging purposes.
* [`sailtrack-boat`](sailtrack/sailtrack-boat) - Python script that processes the incoming sensor data to produce boat metrics (e.g. SOG, COG,...).

All the tasks listed above are run as [systemd](https://en.wikipedia.org/wiki/Systemd) services, which means that systemd will start them, log their output in the system journal, and restart them if they fail.
If you are unfamiliar with systemd, check out one of the many [online tutorials](https://www.digitalocean.com/community/tutorials/systemd-essentials-working-with-services-units-and-the-journal).
Every systemd service is defined by a unit file.
The unit files of the custom scripts are all located in [`rootfs/etc/systemd/system`](rootfs/etc/systemd/system).

## Data Management

### Backing up Data

To back up the InfluxDB database, and therefore all the data present on the system, execute the following commands:

```bash
influxd backup -portable ~/sailtrack_core_db_backup
tar -czf ~/sailtrack_core_db_backup.tar.gz ~/sailtrack_core_db_backup
```

Now download and keep `~/sailtrack_core_db_backup.tar.gz` file containing the backup.

### Restoring Backup

When restoring the backup on a SailTrack Core instance, follow these steps:

1. Download the backup file `sailtrack_core_db_backup.tar.gz` and place it in the home directory.

2. Execute the following commands to extract the backup and import it in influx:

```bash
tar -xzf ~/sailtrack_core_db_backup.tar.gz
influxd restore -portable -db sailtrack-data -newdb sailtrack-data-tmp ~/sailtrack_core_db_backup
```

3. Run the InfluxDB CLI by typing `influx` in the terminal and then execute the following commands within the CLI:

```bash
use sailtrack-data-tmp
SELECT * INTO "sailtrack-data".autogen.:MEASUREMENT FROM "sailtrack-data-tmp".autogen./.*/ GROUP BY *
DROP DATABASE "sailtrack-data-tmp"
```

These commands switch to the temporary database, copy data to the original database, and then drop the temporary database.

4. Remove the unused files:

```bash
rm ~/saitrack_core_db_backup.tar.gz
rm -r ~/sailtrack_core_db_backup
```
