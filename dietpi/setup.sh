apt install telegraf
wget -P /etc/telegraf https://raw.githubusercontent.com/metis-vela-unipd/sailtrack-core/main/telegraf/telegraf.conf
echo '+ telegraf' >> /boot/dietpi/.dietpi-services_include_exclude
dietpi-services restart telegraf

pip3 install git+https://github.com/metis-vela-unipd/sailtrack-core.git
echo '+ sailtrackd' >> /boot/dietpi/.dietpi-services_include_exclude
dietpi-services start sailtrackd
