<p align="center">
  <img src="https://raw.githubusercontent.com/metis-vela-unipd/sailtrack-docs/main/Assets/SailTrack%20Logo.png" width="180">
</p>

<p align="center">
  <img src="https://img.shields.io/github/license/metis-vela-unipd/sailtrack-core" />
  <img src="https://img.shields.io/github/v/release/metis-vela-unipd/sailtrack-core" />
  <img src="https://img.shields.io/github/workflow/status/metis-vela-unipd/sailtrack-core/Publish%20Release" />
</p>

# SailTrack Core

SailTrack Core is the central component of the SailTrack system, it manages connections and gathers data. To learn more about the SailTrack project, please visit the [documentation repository](https://github.com/metis-vela-unipd/sailtrack-docs).

The SailTrack Core module is based on a battery powered Raspberry Pi SBC running a custom version of the Raspberry Pi OS, namely, [DietPi](https://dietpi.com). For a more detailed hardware description of the module, please refer to the [Bill Of Materials](hardware/BOM.csv). The 3D-printable enclosure con be found [here](hardware/STL).

<p align="center">
  <br/>
  <img src="hardware/Connection Diagram.svg">
</p>

## Installation

Follow the instructions below to get the SailTrack Core OS correctly installed. If you encounter any problem, please [open an issue](https://github.com/metis-vela-unipd/sailtrack-core/issues/new).

1. [Download](https://github.com/metis-vela-unipd/sailtrack-core/releases/latest/download/SailTrack-Core_RPi-ARMv8-Bullseye.7z) and extract the latest SailTrack Core OS image.

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

      The `<raspberry-ip-address>` can be found by checking the router administration dashboard or by using a tool such as [Angry IP Scanner](https://angryip.org). The password is the default one (`dietpi`) or the one set in Step 4.

   2. Dismiss the `DietPi first run setup is currently running on another screen` message by hitting <kbd>Ctrl</kbd> + <kbd>C</kbd>.

   3. Check the logs coming from the installation progress with the following command:

      ```
      tail -f /var/tmp/dietpi/logs/dietpi-firstrun-setup.log
      ```

9. Wait until the `SailTrack-CoreNet` WiFi network is visible, meaning that the installation process has been successfully completed.

10. Congratulations! You've successfully installed the SailTrack Core OS.

## Usage

Once the installation process has been successfully completed, you can use SailTrack Core by following the steps below.

1. Power on the module by pressing the power push button. Once the WiFi network has been created, SailTrack Core will start collecting the measurements coming from the external modules.
2. Connect to the `SailTrack-CoreNet` WiFi network with your pc, smartphone, tablet,....
3. Visit http://192.168.42.1:3001 (user: `admin`, password: `dietpi` or the one set in Step 4 of the installation) to connect to the Grafana dashboards to see real-time data and browse the database. To learn more about using Grafana, visit the [official guide](https://grafana.com/docs/grafana/latest/getting-started/getting-started/).

## Contributing

Pull requests are welcome. For major changes, please [open an issue](https://github.com/metis-vela-unipd/sailtrack-core/issues/new) first to discuss what you would like to change.

## License

Copyright © 2022, [Métis Vela Unipd](https://github.com/metis-vela-unipd). SailTrack Core is available under the [GPL-3.0 license](https://www.gnu.org/licenses/gpl-3.0.en.html). See the LICENSE file for more info. 
