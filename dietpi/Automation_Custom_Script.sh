# Install and configure RTC Module (DS3231)
echo dtoverlay=i2c-rtc,ds3231 >> /boot/config.txt

# Install and configure telegraf
apt install telegraf
ln -s /boot/sailtrack/telegraf/* /etc/telegraf/telegraf.d
echo "+ telegraf" >> /boot/dietpi/.dietpi-services_include_exclude

# Install and configure x708
echo "/boot/sailtrack/x708/pwr.sh &" > /var/lib/dietpi/postboot.d/x708pwr.sh
echo "alias x708off='sudo /boot/sailtrack/x708/softsd.sh'" >> /etc/bash.bashrc

# Install and configure sailtrackd
pip3 install -r /boot/sailtrack/requirements.txt
systemctl link /boot/sailtrack/systemd/*
echo "+ sailtrackd" >> /boot/dietpi/.dietpi-services_include_exclude
