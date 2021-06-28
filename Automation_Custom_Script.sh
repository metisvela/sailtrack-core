. /boot/dietpi/func/dietpi-globals

# Configure WiFi Hotspot
/boot/dietpi/func/dietpi-set_hardware wifimodules onboard_enable
/boot/dietpi/func/dietpi-set_hardware wificountrycode "$(sed -n '/^[[:blank:]]*AUTO_SETUP_NET_WIFI_COUNTRY_CODE=/{s/^[^=]*=//p;q}' /boot/dietpi.txt)"

# Configure RTC Module
G_AGP fake-hwclock
G_CONFIG_INJECT "dtoverlay=i2c-rtc,ds3231" "dtoverlay=i2c-rtc,ds3231" /boot/config.txt
G_EXEC sed -i "/systemd/,/fi/s/^/#/" /lib/udev/hwclock-set
G_EXEC sed -i "/--systz/s/^/#/" /lib/udev/hwclock-set

# Install packages
G_AGI install telegraf
G_EXEC pip3 install -r /boot/sailtrack/sailtrackd/requirements.txt

# Merge filesystem
G_EXEC cp -rT /boot/rootfs /

# Enable services
G_CONFIG_INJECT "+ telegraf" "+ telegraf" /boot/dietpi/.dietpi-services_include_exclude
/boot/dietpi/dietpi-services dietpi_controlled telegraf
for s in /boot/rootfs/etc/systemd/system/*.service; do
  service=$(basename "$s" .service)
  G_EXEC systemctl enable "$service"
  G_CONFIG_INJECT "+ $service" "+ $service" /boot/dietpi/.dietpi-services_include_exclude
  /boot/dietpi/dietpi-services dietpi_controlled "$service"
done

# Remove merged filesystem
G_EXEC rm -r /boot/rootfs

# Configure DietPi Banner
index=0
for i in 1 1 1 0 0 1 0 0 0 0 0 0 0 0; do
  echo "aENABLED[$((index++))]=$i" >> /boot/dietpi/.dietpi-banner
done

# Accept DietPi License
G_EXEC rm /var/lib/dietpi/license.txt

# Remove DietPi-VPN
G_EXEC rm /etc/systemd/system/dietpi-vpn.service

# Reboot
(while [ "$(</boot/dietpi/.install_stage)" != 2 ]; do sleep 1; done; /usr/sbin/reboot) > /dev/null 2>&1 &
exit 1
