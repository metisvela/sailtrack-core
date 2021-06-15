. /boot/dietpi/func/dietpi-globals

# Configure WiFi Hotspot
/boot/dietpi/func/dietpi-set_hardware wifimodules onboard_enable
/boot/dietpi/func/dietpi-set_hardware wificountrycode "$(sed -n '/^[[:blank:]]*AUTO_SETUP_NET_WIFI_COUNTRY_CODE=/{s/^[^=]*=//p;q}' /boot/dietpi.txt)"

# Install and configure RTC Module (DS3231)
G_CONFIG_INJECT "dtoverlay=i2c-rtc,ds3231" "dtoverlay=i2c-rtc,ds3231" /boot/config.txt
G_AGP fake-hwclock

# Install and configure telegraf
G_AGI install telegraf
G_EXEC ln -s /boot/sailtrack/telegraf/* /etc/telegraf/telegraf.d
G_CONFIG_INJECT "+ telegraf" "+ telegraf" /boot/dietpi/.dietpi-services_include_exclude

# Install and configure x708
echo "/boot/sailtrack/x708/pwr.sh &" > /var/lib/dietpi/postboot.d/x708pwr.sh
echo "alias poweroff='/boot/sailtrack/x708/softsd.sh'" > /etc/bashrc.d/x708softsd.sh

# Install and configure sailtrackd
G_EXEC pip3 install -r /boot/sailtrack/requirements.txt
G_EXEC systemctl link /boot/sailtrack/systemd/*
G_CONFIG_INJECT "+ sailtrackd" "+ sailtrackd" /boot/dietpi/.dietpi-services_include_exclude

# Change network policy
G_CONFIG_INJECT "CONFIG_BOOT_WAIT_FOR_NETWORK=" "CONFIG_BOOT_WAIT_FOR_NETWORK=1" /boot/dietpi.txt

# Reboot
(while [ "$(</boot/dietpi/.install_stage)" != 2 ]; do sleep 1; done; /usr/sbin/reboot) > /dev/null 2>&1 &
