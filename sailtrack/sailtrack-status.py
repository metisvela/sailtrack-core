#!/usr/bin/env python3

import json
import logging
import os
import struct
import sys
from datetime import timedelta

from gpiozero import DigitalInputDevice, CPUTemperature, LoadAverage, DiskUsage
from paho.mqtt.client import Client
from smbus2 import SMBus
from timeloop import Timeloop

# -------------------------- Configuration -------------------------- #

MQTT_PUBLISH_FREQ_HZ = 0.1
LOG_PRINT_FREQ_HZ = 0.1

MQTT_HOST_ADDR = "192.168.42.1"
MQTT_USERNAME = "mosquitto"
MQTT_PASSWORD = "dietpi"

I2C_BUS_NUM = 1
I2C_ADDR = 0x36
I2C_BATTERY_VOLTAGE_REG = 0x02
I2C_BATTERY_CAPACITY_REG = 0x04

CHARGE_PIN = 6

MQTT_JOB_INTERVAL_SEC = 1 / MQTT_PUBLISH_FREQ_HZ
LOG_JOB_INTERVAL_SEC = 1 / LOG_PRINT_FREQ_HZ

# ------------------------------------------------------------------- #

published_messages = 0
received_messages = 0


def on_publish_callback(client, userdata, mid):
    global published_messages
    published_messages += 1


def get_battery_voltage():
    regval = bus.read_word_data(I2C_ADDR, I2C_BATTERY_VOLTAGE_REG)
    regval = struct.unpack("<H", struct.pack(">H", regval))[0]
    return regval * 1.25 / 1000 / 16


def get_battery_capacity():
    regval = bus.read_word_data(I2C_ADDR, I2C_BATTERY_CAPACITY_REG)
    regval = struct.unpack("<H", struct.pack(">H", regval))[0]
    return regval / 256


bus = SMBus(I2C_BUS_NUM)
mqtt = Client("sailtrack-status")
mqtt.username_pw_set(MQTT_USERNAME, MQTT_PASSWORD)
mqtt.on_publish = on_publish_callback
mqtt.connect(MQTT_HOST_ADDR)
tl = Timeloop()
formatter = logging.Formatter("[%(name)s] [%(levelname)s] %(message)s")
logging.getLogger("timeloop").handlers[0].setFormatter(formatter)
logger = logging.getLogger("log_job")
logger.setLevel(logging.INFO)
handler = logging.StreamHandler()
handler.setFormatter(formatter)
logger.addHandler(handler)


@tl.job(interval=timedelta(seconds=MQTT_JOB_INTERVAL_SEC))
def mqtt_job():
    sys.stdout = open(os.devnull, 'w')
    mqtt.publish("status/core", json.dumps({
        "battery": {
            "voltage": get_battery_voltage(),
            "capacity": get_battery_capacity(),
            "charging": not DigitalInputDevice(CHARGE_PIN).value
        },
        "cpu": {
            "temperature": CPUTemperature().temperature,
            "load": LoadAverage().load_average
        },
        "disk": {
            "load": DiskUsage().value
        }
    }))
    sys.stdout = sys.__stdout__


@tl.job(interval=timedelta(seconds=LOG_JOB_INTERVAL_SEC))
def log_job():
    logger.info(f"Published messages: {published_messages}, Received messages: {received_messages}")


tl.start(block=True)
