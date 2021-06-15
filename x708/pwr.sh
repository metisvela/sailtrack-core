#!/bin/bash

SHUTDOWN=5
REBOOTPULSEMINIMUM=200
REBOOTPULSEMAXIMUM=600
echo "$SHUTDOWN" > /sys/class/gpio/export
echo "in" > /sys/class/gpio/gpio$SHUTDOWN/direction
BOOT=12
echo "$BOOT" > /sys/class/gpio/export
echo "out" > /sys/class/gpio/gpio$BOOT/direction
echo "1" > /sys/class/gpio/gpio$BOOT/value

while : ; do
  shutdownSignal=$(</sys/class/gpio/gpio$SHUTDOWN/value)
  if [ "$shutdownSignal" = 0 ]; then /bin/sleep 0.2
  else
    pulseStart=$(date +%s%N | cut -b1-13)
    while [ "$shutdownSignal" = 1 ]; do
      /bin/sleep 0.02
      if [ $(($(date +%s%N | cut -b1-13)-"$pulseStart")) -gt $REBOOTPULSEMAXIMUM ]; then
        sudo /usr/sbin/poweroff
        exit
      fi
      shutdownSignal=$(</sys/class/gpio/gpio$SHUTDOWN/value)
    done
    if [ $(($(date +%s%N | cut -b1-13)-"$pulseStart")) -gt $REBOOTPULSEMINIMUM ]; then
      sudo /usr/sbin/reboot
      exit
    fi
  fi
done
