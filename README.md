<p align="center">
  <img src="https://raw.githubusercontent.com/metisvela/sailtrack/main/assets/sailtrack-logo.svg" width="180">
</p>

<p align="center">
  <img src="https://img.shields.io/github/license/metisvela/sailtrack-core" />
  <img src="https://img.shields.io/github/v/release/metisvela/sailtrack-core" />
  <img src="https://img.shields.io/github/actions/workflow/status/metisvela/sailtrack-core/publish.yml" />
</p>

# SailTrack Core

SailTrack Core is the central component of the SailTrack system, it manages connections and gathers data. To learn more about the SailTrack project, please visit the [project repository](https://github.com/metisvela/sailtrack).

The SailTrack Core module is based on a battery powered Raspberry Pi SBC running a custom version of the Raspberry Pi OS, namely, [DietPi](https://dietpi.com). For a more detailed hardware description of the module, please refer to the [Bill Of Materials](hardware/BOM.csv). The 3D-printable enclosure con be found [here](hardware/STL).

The module performs the following tasks:

* It creates the SailTrack Network, the Wi-Fi network needed by all the modules to communicate.
* It acts as the [MQTT](https://mqtt.org) Broker, managing the exchange of MQTT messages between modules.
* It runs the [InfluxDB](https://www.influxdata.com) database, gathering all the measurements coming from the sensors.
* It runs the [Grafana](https://grafana.com) server, for the visualization of real-time and logged metrics.
* It processes the readings coming from the sensors by filtering and combining them to obtain derived metrics.

<p align="center">
  <br/>
  <img src="hardware/Connection Diagram.svg">
</p>

![module-image](hardware/Module%20Image.jpg)

## Installation

Follow the instructions below to get the SailTrack Core OS correctly installed. If you encounter any problem, please [open an issue](https://github.com/metisvela/sailtrack-core/issues/new).

1. [Download](https://github.com/metisvela/sailtrack-core/releases/latest/download/SailTrack-Core_RPi-ARMv8-Bullseye.7z) and extract the latest SailTrack Core OS image.
2. Insert the Raspberry Pi microSD card into the computer.
3. Flash the downloaded `.img` file into the SD card using a flashing tool such as [balenaEtcher](https://www.balena.io/etcher/).
4. **(OPTIONAL)** Change the passwords from the default ones by modifying the `AUTO_SETUP_GLOBAL_PASSWORD` and the `SOFTWARE_WIFI_HOTSPOT_KEY` in the `dietpi.txt` file located inside the SD card.
5. Eject the SD card from the computer and insert it into the Raspberry Pi.
6. Connect the Raspberry Pi to internet with an ethernet cable.
7. Power on the Raspberry Pi. The first run setup will automatically start to download and configure the required packages. *Note: this might take a few minutes, depending on the internet connection quality, follow the next step to check the installation progress.*
8. **(OPTIONAL)** Check the installation progress:
   1. Connect to the Raspberry Pi using a device connected to the same network:
      ```
      ssh root@<raspberry-ip-address>
      ```
      The `<raspberry-ip-address>` can be found by checking the router administration dashboard or by using a tool such as [Angry IP Scanner](https://angryip.org). The password is the default one (`sailtrack`) or the one set in Step 4.
   2. Dismiss the `DietPi first run setup is currently running on another screen` message by hitting <kbd>Ctrl</kbd> + <kbd>C</kbd>.
   3. Check the logs coming from the installation progress with the following command:
      ```
      tail -f /var/tmp/dietpi/logs/*.log
      ```
9. Wait until the `SailTrack-CoreNet` Wi-Fi network is visible, meaning that the installation process has been successfully completed.

## Usage

Once the installation process has been successfully completed, you can use SailTrack Core by following the steps below.

1. Power on the module by pressing the power push button. Once the Wi-Fi network has been created, the other modules will automatically power on and SailTrack Core will start collecting the measurements coming from them. *Note: the automatic power on process might take a few minutes to complete, once a module is connected to the network the onboard LED will emit a steady light.*
2. Connect to the `SailTrack-CoreNet` Wi-Fi network with your pc, smartphone, tablet,... (password: `sailtracknet` or the one set in Step 4 of the installation).
3. Visit http://192.168.42.1:3001 (user: `admin`, password: `sailtrack` or the one set in Step 4 of the installation) to connect to the Grafana dashboards to see real-time data and browse the database. To learn more about using Grafana, visit the [official guide](https://grafana.com/docs/grafana/latest/getting-started/getting-started/).
4. To power off the system press and hold the power push button of the Core module until the power light starts blinking. Once the power light turns off, all the other modules will automatically turn off.

## Contributing

Contributors are welcome. If you are a student of the University of Padova, please apply for the Metis Sailing Team in the [website](http://metisvela.dii.unipd.it), specifying in the application form that you are interested in contributing to the SailTrack Project. If you are not a student of the University of Padova, feel free to open Pull Requests and Issues to contribute to the project.

To learn more about contributing to this repository, check out the [Developer's Guide](DEVELOPER.md).

## License

Copyright © 2023, [Metis Sailing Team](https://github.com/metisvela). SailTrack Core is available under the [GPL-3.0 license](https://www.gnu.org/licenses/gpl-3.0.en.html). See the LICENSE file for more info. 
