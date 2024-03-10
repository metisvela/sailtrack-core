This is the Work-in-Progress project for the Kalman Filter implementation in C/C++. At the moment 2 things can be executed:

First (finished). A test script that uses logged data and runs the Kalman Filter offline. Data is taken and written to the test data folder. Fully working for the given data files.

To compile & run:

- make sure you have the Eigen library installed https://eigen.tuxfamily.org/ and added the library path in the file "matrix-types.h" and "eigen-2-csd.h".
- switch to the correct kalman sampling time in "model.h" for the logged data
- to compile: `g++ test-script.cpp preprocessing-logged.cpp eigen-2-csv.cpp`
- run the executable: (on linux) `./a.out`
                      (on Windows) `start a.exe`


Second (WiP). A main program that eventually will run the Kalman Filter online on the Raspbarry Pi, by connecting to the mqtt broker, subscribing to sensor topics, running the filter and publishing the results.
This is a Work-in-Progress. In particular, this version successfully connects to a MQTT broker using sensor simulation data.

To compile & run:

- make sure you have the Eigen library installed and added the library path in the file "matrix-types.h" and "eigen-2-csd.h".
- install the libmosquitto library (should come with the mosquitto installation https://mosquitto.org/)
- switch to the correct kalman sampling time in "model.h" for the live data
- to compile: `g++ main.cpp -lmosquitto`
- run the executable: (on linux) `./a.out`

To test with sensor simulation script on your machine:

- for all configurations, use "localhost" as address
- create mqtt broker in docker container: `docker run -it -p 1883:1883 eclipse-mosquitto mosquitto -c /mosquitto-no-auth.conf`
- run sensor simulation script from boat-simulations repo: `python3 sensor_simulation_mqtt/mqtt-sensor-simulation.py`
- run compiled C++ executable from above
- check connection using MQTTExplorer
