#!/bin/bash

# `Automation_Custom_Script.sh` - Script that runs on the first boot of the OS. It finishes the installation and sets up
# the necessary components for SailTrack. The logs of the script are automatically logged in
# `/var/tmp/dietpi/logs/dietpi-firstrun-setup.log`.

# Load DietPi functions (G_* functions)
. /boot/dietpi/func/dietpi-globals

# Alias the `poweroff` command to trigger the X708 HAT software shutdown instead
G_CONFIG_INJECT "alias poweroff=/boot/sailtrack/sailtrack-x708_softsd" "alias poweroff=/boot/sailtrack/sailtrack-x708_softsd" /etc/bash.bashrc

# Remove unused components
G_EXEC rm /etc/systemd/system/dietpi-vpn.service
G_EXEC rm /etc/systemd/system/dietpi-cloudshell.service

# Install required packages
G_AGI telegraf rsync
G_EXEC_OUTPUT=1 G_EXEC_OUTPUT_COL="\e[90m" G_EXEC pip3 install gpiozero paho-mqtt smbus2 timeloop

# Enable SailTrack services
G_EXEC systemctl enable sailtrack-x708_pwr
G_CONFIG_INJECT "+ telegraf" "+ telegraf" /boot/dietpi/.dietpi-services_include_exclude
G_CONFIG_INJECT "+ sailtrack-status" "+ sailtrack-status" /boot/dietpi/.dietpi-services_include_exclude
G_CONFIG_INJECT "+ sailtrack-timesync" "+ sailtrack-timesync" /boot/dietpi/.dietpi-services_include_exclude
G_CONFIG_INJECT "+ sailtrack-tileserver" "+ sailtrack-tileserver" /boot/dietpi/.dietpi-services_include_exclude
G_CONFIG_INJECT "+ sailtrack-boat" "+ sailtrack-boat" /boot/dietpi/.dietpi-services_include_exclude
G_EXEC /boot/dietpi/dietpi-services enable sailtrack-status
G_EXEC /boot/dietpi/dietpi-services enable sailtrack-timesync
G_EXEC /boot/dietpi/dietpi-services enable sailtrack-tileserver
G_EXEC /boot/dietpi/dietpi-services enable sailtrack-boat

# Configure DietPi Banner
G_EXEC touch /boot/dietpi/.dietpi-banner
banner_settings=(1 1 1 0 0 1 0 1 0 0 0 0 0 0 0 0)
for i in "${!banner_settings[@]}"; do
  G_CONFIG_INJECT "aENABLED\[$i]=" "aENABLED[$i]=${banner_settings[$i]}" /boot/dietpi/.dietpi-banner
done

# Configure passwords and generate keys
G_EXEC /boot/dietpi/dietpi-services restart grafana-server
global_password=$(openssl enc -d -a -md sha256 -aes-256-cbc -iter 10000 -salt -pass pass:'DietPiRocks!' -in /var/lib/dietpi/dietpi-software/.GLOBAL_PW.bin)
grafana_service_account_id=$(\
  curl --retry 10 --retry-delay 5 --retry-connrefused -s -X POST -H "Content-Type: application/json" -d '{"name":"SailTrack", "role": "Admin"}' "http://admin:$global_password@localhost:3001/api/serviceaccounts" | \
  python3 -c "import sys, json; print(json.load(sys.stdin)['id'])" \
)
grafana_api_key=$(\
  curl --retry 10 --retry-delay 5 --retry-connrefused -s -X POST -H "Content-Type: application/json" -d '{"name":"Telegraf"}' "http://admin:$global_password@localhost:3001/api/serviceaccounts/$grafana_service_account_id/tokens" | \
  python3 -c "import sys, json; print(json.load(sys.stdin)['key'])" \
)
GCI_PASSWORD=1 G_CONFIG_INJECT "SAILTRACK_GLOBAL_PASSWORD=" "SAILTRACK_GLOBAL_PASSWORD=$global_password" /etc/default/sailtrack
GCI_PASSWORD=1 G_CONFIG_INJECT "SAILTRACK_GLOBAL_PASSWORD=" "SAILTRACK_GLOBAL_PASSWORD=$global_password" /etc/default/telegraf
GCI_PASSWORD=1 G_CONFIG_INJECT "SAILTRACK_GRAFANA_API_KEY=" "SAILTRACK_GRAFANA_API_KEY=$grafana_api_key" /etc/default/telegraf

# Generate the default telegraf configuration (the SailTrack configuration is loaded as a drop-in file under
# `/etc/telegraf/telegraf.d`)
telegraf --section-filter=agent config > /etc/telegraf/telegraf.conf
# Remove the default "host" tag from the telegraf metrics
G_CONFIG_INJECT "omit_hostname =" "  omit_hostname = true" /etc/telegraf/telegraf.conf

# Create the "SailTrack" organization (required by Grafana)
G_EXEC curl --retry 10 --retry-delay 5 --retry-connrefused -s -X PUT -H "Content-Type: application/json" -d '{"name":"SailTrack"}' "http://admin:$global_password@localhost:3001/api/org"
# Install the third-party custom track map panel
G_EXEC /usr/share/grafana/bin/grafana cli --pluginUrl=https://github.com/alexandrainst/alexandra-trackmap-panel/archive/master.zip plugins install alexandra-trackmap-panel
# Allow the third-party plugin
G_CONFIG_INJECT ";allow_loading_unsigned_plugins =" "allow_loading_unsigned_plugins = alexandra-trackmap-panel" /etc/grafana/grafana.ini
# Enable the insertion of SVG files in Grafana panels (needed by the SailTrack logo in the home dashboard)
G_CONFIG_INJECT ";disable_sanitize_html =" "disable_sanitize_html = true" /etc/grafana/grafana.ini
# Set the default home dashboard to the SailTrack one
G_CONFIG_INJECT ";default_home_dashboard_path =" "default_home_dashboard_path = /usr/share/sailtrack/dashboards/sailtrack-home.json" /etc/grafana/grafana.ini

# Reboot after first boot is completed
(while [ -f "/root/AUTO_CustomScript.sh" ]; do sleep 1; done; /usr/sbin/reboot) > /dev/null 2>&1 &
