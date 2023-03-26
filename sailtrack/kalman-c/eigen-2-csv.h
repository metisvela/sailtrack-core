#ifndef EIGEN_2_CSV_H
#define EIGEN_2_CSV_H

#include <iostream>
#include <fstream>
#include </usr/include/eigen3/Eigen/Dense> //TODO: fix path location

Eigen::MatrixXd loadData(std::string fileToOpen);
void saveData(std::string fileName, Eigen::MatrixXd  matrix);
void saveData(std::string fileName, std::vector<Eigen::VectorXd> data); //NOTE - saves eigen vectors as rows

#endif //EIGEN_2_CSV_H
