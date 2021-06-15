#!/bin/bash

if [ "$EUID" -ne 0 ]
  then echo "Please run as root"
  exit
fi

BUTTON=13

echo "$BUTTON" > /sys/class/gpio/export
echo "out" > /sys/class/gpio/gpio$BUTTON/direction
echo "1" > /sys/class/gpio/gpio$BUTTON/value

SLEEP=${1:-4}

re='^[0-9\.]+$'
if ! [[ $SLEEP =~ $re ]] ; then
   echo "error: sleep time not a number" >&2; exit 1
fi

/bin/sleep "$SLEEP"

# Restore GPIO 13
echo "0" > /sys/class/gpio/gpio$BUTTON/value
