cd /boot/sailtrack || exit

# Install and configure telegraf
apt install telegraf
ln telegraf/* /etc/telegraf/telegraf.d
echo "+ telegraf" >> /boot/dietpi/.dietpi-services_include_exclude

# Install and configure x708
echo "/boot/sailtrack/x708/pwr.sh &" > /var/lib/dietpi/postboot.d/x708pwr.sh
echo "alias x708off='sudo /boot/sailtrack/x708/softsd.sh'" >> /etc/bash.bashrc

# Install and configure sailtrackd
pip3 install -r requirements.txt
systemctl link systemd/*
echo "+ sailtrackd" >> /boot/dietpi/.dietpi-services_include_exclude

reboot
