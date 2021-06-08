# Enable RTC module
echo dtoverlay=i2c-rtc,ds3231 >> /boot/config.txt

# Install and configure Telegraf
apt install telegraf
wget -P /etc/telegraf https://raw.githubusercontent.com/metis-vela-unipd/sailtrack-core/develop/telegraf/telegraf.conf
echo '+ telegraf' >> /boot/dietpi/.dietpi-services_include_exclude
dietpi-services restart telegraf

# Install and configure sailtrackd
pip3 install git+https://github.com/metis-vela-unipd/sailtrack-core.git
echo '+ sailtrackd' >> /boot/dietpi/.dietpi-services_include_exclude
dietpi-services start sailtrackd

reboot
