This is the work-in-progress repository for the Kalma Filter implementation in C/C++. At the moment 2 things can be executed:

First (finished). A test script that uses logged data and runs the Kalman Filter offline. Data is taken and written to the test data folder. Fully working for the given data files.

To compile & run:

- make sure you have the Eigen library installed https://eigen.tuxfamily.org/ and added the library path in the file "matrix-types.h"
- switch to the correct kalman sampling time in "model.h" for the logged data
- to compile: g++ test-script.cpp preprocessing-logged.cpp eigen-2-csv.cpp
- run the executable: (on linux) ./a.out


Second (WiP). A main program that eventually will run the Kalman Filter online on the Raspbarry Pi, by connecting to the mqtt broker, subscribing to sensor topics, running the filter and publishing the results.
This is a Work-in-Progress. In particular, the first code draft is done, and needs now to be tested in an online environment.

To compile & run:

- make sure you have the Eigen library installed and added the library path in the file "matrix-types.h"
- install the libmosquitto library (should come with the mosquitto installation https://mosquitto.org/)
- switch to the correct kalman sampling time in "model.h" for the logged data
- to compile: g++ main.cpp -lmosquitto
- run the executable: (on linux) ./a.out

Bugs:
- not connecting to mqtt broker