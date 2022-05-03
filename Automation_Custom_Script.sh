. /boot/dietpi/func/dietpi-globals

# Configure X708 HAT
G_CONFIG_INJECT "alias poweroff=/boot/sailtrack/sailtrack-x708_softsd" "alias poweroff=/boot/sailtrack/sailtrack-x708_softsd" /etc/bash.bashrc

# Remove unused components
G_EXEC rm /etc/systemd/system/dietpi-vpn.service
G_EXEC rm /etc/systemd/system/dietpi-cloudshell.service

# Install required packages
G_AGI telegraf
G_EXEC_OUTPUT=1 G_EXEC_OUTPUT_COL="\e[90m" G_EXEC pip3 install gpiozero paho-mqtt smbus2 timeloop

# Enable services
G_EXEC systemctl enable sailtrack-x708_asd
G_EXEC systemctl enable sailtrack-x708_pwr
G_CONFIG_INJECT "+ telegraf" "+ telegraf" /boot/dietpi/.dietpi-services_include_exclude
G_CONFIG_INJECT "+ sailtrack-status" "+ sailtrack-status" /boot/dietpi/.dietpi-services_include_exclude
G_CONFIG_INJECT "+ sailtrack-timesync" "+ sailtrack-timesync" /boot/dietpi/.dietpi-services_include_exclude
G_EXEC /boot/dietpi/dietpi-services dietpi_controlled telegraf
G_EXEC /boot/dietpi/dietpi-services dietpi_controlled sailtrack-status
G_EXEC /boot/dietpi/dietpi-services dietpi_controlled sailtrack-timesync

# Configure DietPi Banner
G_EXEC touch /boot/dietpi/.dietpi-banner
settings=(1 1 1 0 0 1 0 1 0 0 0 0 0 0 0 0)
for i in "${!settings[@]}"; do
  G_CONFIG_INJECT "aENABLED\[$i]=" "aENABLED[$i]=${settings[$i]}" /boot/dietpi/.dietpi-banner
done

# Reboot after first boot is completed
(while [ -f "/root/AUTO_CustomScript.sh" ]; do sleep 1; done; /usr/sbin/reboot) > /dev/null 2>&1 &
