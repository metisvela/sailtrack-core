#!/bin/bash

. /boot/dietpi/func/dietpi-globals

FP_EXIT_CODE="/run/dietpi/.timesync_exit_status"

if [[ -f $FP_EXIT_CODE && $(<$FP_EXIT_CODE) -eq 0 ]]; then
  G_CONFIG_INJECT "CONFIG_BOOT_WAIT_FOR_NETWORK=" "CONFIG_BOOT_WAIT_FOR_NETWORK=0" /boot/dietpi.txt
  G_CONFIG_INJECT "CONFIG_NTP_MODE=" "CONFIG_NTP_MODE=0" /boot/dietpi.txt
  rm $FP_EXIT_CODE
fi
