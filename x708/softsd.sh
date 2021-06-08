#!/bin/bash

BUTTON=13

echo "$BUTTON" > /sys/class/gpio/export;
echo "out" > /sys/class/gpio/gpio$BUTTON/direction
echo "1" > /sys/class/gpio/gpio$BUTTON/value

SLEEP=${1:-4}

re='^[0-9\.]+$'
if ! [[ $SLEEP =~ $re ]] ; then
   echo "error: sleep time not a number" >&2; exit 1
fi

echo "X708 Shutting down..."
/bin/sleep $SLEEP

#restore GPIO 13
echo "0" > /sys/class/gpio/gpio$BUTTON/value
