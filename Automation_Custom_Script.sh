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
G_EXEC rm -r /boot/rootfs

# Enable services
for s in /etc/systemd/system/sailtrack*.service; do G_EXEC systemctl enable "$s"; done
G_CONFIG_INJECT "+ telegraf" "+ telegraf" /boot/dietpi/.dietpi-services_include_exclude
for s in /etc/systemd/system/sailtrack-sailtrackd*.service; do
  servicename=$(basename "$s" .service)
  G_CONFIG_INJECT "+ $servicename" "+ $servicename" /boot/dietpi/.dietpi-services_include_exclude
  /boot/dietpi/dietpi-services dietpi_controlled "$servicename"
done
/boot/dietpi/dietpi-services dietpi_controlled telegraf

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
