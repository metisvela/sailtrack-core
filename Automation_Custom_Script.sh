. /boot/dietpi/func/dietpi-globals

# Configure X708 HAT
G_CONFIG_INJECT "alias poweroff=/boot/sailtrack/sailtrack-x708_softsd" "alias poweroff=/boot/sailtrack/sailtrack-x708_softsd" /etc/bash.bashrc

# Remove unused components
G_EXEC rm /etc/systemd/system/dietpi-vpn.service
G_EXEC rm /etc/systemd/system/dietpi-cloudshell.service

# Install required packages
G_AGI telegraf
G_EXEC_OUTPUT=1 G_EXEC_OUTPUT_COL="\e[90m" G_EXEC pip3 install gpiozero paho-mqtt smbus2 timeloop numpy filterpy

# Enable services
G_EXEC systemctl enable sailtrack-x708_pwr
G_CONFIG_INJECT "+ telegraf" "+ telegraf" /boot/dietpi/.dietpi-services_include_exclude
G_CONFIG_INJECT "+ sailtrack-status" "+ sailtrack-status" /boot/dietpi/.dietpi-services_include_exclude
G_CONFIG_INJECT "+ sailtrack-timesync" "+ sailtrack-timesync" /boot/dietpi/.dietpi-services_include_exclude
G_CONFIG_INJECT "+ sailtrack-tileserver" "+ sailtrack-tileserver" /boot/dietpi/.dietpi-services_include_exclude
G_CONFIG_INJECT "+ sailtrack-processor" "+ sailtrack-processor" /boot/dietpi/.dietpi-services_include_exclude
G_EXEC /boot/dietpi/dietpi-services enable telegraf
G_EXEC /boot/dietpi/dietpi-services start telegraf
G_EXEC /boot/dietpi/dietpi-services enable sailtrack-status
G_EXEC /boot/dietpi/dietpi-services start sailtrack-status
G_EXEC /boot/dietpi/dietpi-services enable sailtrack-timesync
G_EXEC /boot/dietpi/dietpi-services start sailtrack-timesync
G_EXEC /boot/dietpi/dietpi-services enable sailtrack-tileserver
G_EXEC /boot/dietpi/dietpi-services start sailtrack-tileserver
G_EXEC /boot/dietpi/dietpi-services enable sailtrack-processor
G_EXEC /boot/dietpi/dietpi-services start sailtrack-processor

# Configure DietPi Banner
G_EXEC touch /boot/dietpi/.dietpi-banner
SETTINGS=(1 1 1 0 0 1 0 1 0 0 0 0 0 0 0 0)
for i in "${!SETTINGS[@]}"; do
  G_CONFIG_INJECT "aENABLED\[$i]=" "aENABLED[$i]=${SETTINGS[$i]}" /boot/dietpi/.dietpi-banner
done

# Configure passwords and keys
G_EXEC /boot/dietpi/dietpi-services restart grafana-server
GLOBAL_PASSWORD=$(openssl enc -d -a -md sha256 -aes-256-cbc -iter 10000 -salt -pass pass:'DietPiRocks!' -in /var/lib/dietpi/dietpi-software/.GLOBAL_PW.bin)
SERVICE_ACCOUNT_ID=$(\
  curl --retry 10 --retry-delay 5 --retry-connrefused -s -X POST -H "Content-Type: application/json" -d '{"name":"telegraf", "role": "Admin"}' "http://admin:$GLOBAL_PASSWORD@localhost:3001/api/serviceaccounts" | \
  python3 -c "import sys, json; print(json.load(sys.stdin)['id'])" \
)
GRAFANA_API_KEY=$(\
  curl --retry 10 --retry-delay 5 --retry-connrefused -s -X POST -H "Content-Type: application/json" -d '{"name":"telegraf-token"}' "http://admin:$GLOBAL_PASSWORD@localhost:3001/api/serviceaccounts/$SERVICE_ACCOUNT_ID/tokens" | \
  python3 -c "import sys, json; print(json.load(sys.stdin)['key'])" \
)
GCI_PASSWORD=1 G_CONFIG_INJECT "SAILTRACK_GLOBAL_PASSWORD=" "SAILTRACK_GLOBAL_PASSWORD=$GLOBAL_PASSWORD" /etc/default/sailtrack
GCI_PASSWORD=1 G_CONFIG_INJECT "SAILTRACK_GLOBAL_PASSWORD=" "SAILTRACK_GLOBAL_PASSWORD=$GLOBAL_PASSWORD" /etc/default/telegraf
GCI_PASSWORD=1 G_CONFIG_INJECT "SAILTRACK_GRAFANA_API_KEY=" "SAILTRACK_GRAFANA_API_KEY=$GRAFANA_API_KEY" /etc/default/telegraf

# Configure Telegraf
telegraf --section-filter=agent config > /etc/telegraf/telegraf.conf
G_CONFIG_INJECT "omit_hostname =" "  omit_hostname = true" /etc/telegraf/telegraf.conf

# Configure Grafana
G_EXEC curl --retry 10 --retry-delay 5 --retry-connrefused -s -X PUT -H "Content-Type: application/json" -d '{"name":"SailTrack"}' "http://admin:$GLOBAL_PASSWORD@localhost:3001/api/org"
G_EXEC grafana-cli --pluginUrl=https://github.com/alexandrainst/alexandra-trackmap-panel/archive/master.zip plugins install alexandra-trackmap-panel
G_CONFIG_INJECT ";allow_loading_unsigned_plugins =" "allow_loading_unsigned_plugins = alexandra-trackmap-panel" /etc/grafana/grafana.ini
G_CONFIG_INJECT ";disable_sanitize_html =" "disable_sanitize_html = true" /etc/grafana/grafana.ini
G_CONFIG_INJECT ";default_home_dashboard_path =" "default_home_dashboard_path = /usr/share/sailtrack/dashboards/sailtrack-home.json" /etc/grafana/grafana.ini

# Reboot after first boot is completed
(while [ -f "/root/AUTO_CustomScript.sh" ]; do sleep 1; done; /usr/sbin/reboot) > /dev/null 2>&1 &
