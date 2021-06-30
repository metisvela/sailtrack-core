#!/bin/bash

POWEROFF_VOLTAGE=2.5

while : ; do
  read=$(i2cget -y 1 0x36 0x02 w)
  swapped=${read:4:2}${read:2:2}
  voltage=$(awk 'BEGIN {printf "%.2f", ARGV[1] * 1.25 / 1000 / 16}' "$(( 16#$swapped ))")
  if awk "BEGIN {exit ARGV[1] > ARGV[2]}" "$voltage" "$POWEROFF_VOLTAGE"; then sudo /usr/sbin/poweroff; fi
  sleep 10
done
