import json
import logging
import struct
from datetime import timedelta

import RPi.GPIO as GPIO
import smbus
from paho.mqtt.client import Client
from timeloop import Timeloop


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
    swapped = struct.unpack("<H", struct.pack(">H", read))[0]
    capacity = swapped / 256
    return capacity


bus = smbus.SMBus(1)
GPIO.setmode(GPIO.BCM)
GPIO.setup(6, GPIO.IN)
mqtt = Client('core-status')
mqtt.username_pw_set('mosquitto', 'dietpi')
mqtt.on_publish = on_publish
mqtt.connect('localhost')

tl = Timeloop()

logging.getLogger('timeloop').removeHandler(logging.getLogger('timeloop').handlers[0])
logging.basicConfig(format='[%(name)s] [%(levelname)s] %(message)s', level=logging.INFO)
published_messages = 0


@tl.job(interval=timedelta(seconds=10))
def publish_job():
    mqtt.publish('module/core', json.dumps({
        'measurement': 'core',
        'battery': {
            'voltage': read_voltage(bus),
            'capacity': read_capacity(bus),
            'charging': not GPIO.input(6)
        }
    }))


@tl.job(interval=timedelta(seconds=10))
def log_job():
    logging.getLogger('log_job').info(f"Published Messages: {published_messages}")


tl.start(block=True)
