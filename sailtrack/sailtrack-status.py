#!/usr/bin/env python3

import json
import logging
import os
import struct
import sys
from configparser import ConfigParser
from datetime import timedelta

import smbus
from gpiozero import DigitalInputDevice, CPUTemperature, LoadAverage, DiskUsage
from paho.mqtt.client import Client
from timeloop import Timeloop

PUBLISH_RATE = 0.1
LOG_RATE = 0.1


def on_publish(client, userdata, mid):
    global published_messages
    published_messages += 1


def read_voltage(bus):
    address = 0x36
    read = bus.read_word_data(address, 2)
    swapped = struct.unpack('<H', struct.pack('>H', read))[0]
    voltage = swapped * 1.25 / 1000 / 16
    return voltage


def read_capacity(bus):
    address = 0x36
    read = bus.read_word_data(address, 4)
    swapped = struct.unpack('<H', struct.pack('>H', read))[0]
    capacity = swapped / 256
    return capacity


bus = smbus.SMBus(1)
mqtt = Client('core-status')
mqtt.username_pw_set('mosquitto', 'dietpi')
mqtt.on_publish = on_publish
mqtt.connect('localhost')

tl = Timeloop()

logging.getLogger('timeloop').removeHandler(logging.getLogger('timeloop').handlers[0])
logging.basicConfig(format='[%(name)s] [%(levelname)s] %(message)s', level=logging.INFO)
published_messages = 0


@tl.job(interval=timedelta(seconds=1/PUBLISH_RATE))
def publish_job():
    sys.stdout = open(os.devnull, 'w')
    mqtt.publish('module/core', json.dumps({
        'battery': {
            'voltage': read_voltage(bus),
            'capacity': read_capacity(bus),
            'charging': not DigitalInputDevice(6).value
        },
        'cpu': {
            'temperature': CPUTemperature().temperature,
            'load': LoadAverage().load_average
        },
        'disk': {
            'usage': DiskUsage().usage
        },
        'measurement': 'core'
    }))
    sys.stdout = sys.__stdout__


@tl.job(interval=timedelta(seconds=1/LOG_RATE))
def log_job():
    logging.getLogger('log_job').info(f"Published Messages: {published_messages}")


tl.start(block=True)
