#include "eigen-2-csv.h"

//Function to load data from file
Eigen::MatrixXd csv2eigen(std::string fileToOpen)
{
    // Copy&Paste from https://aleksandarhaber.com/eigen-matrix-library-c-tutorial-saving-and-loading-data-in-from-a-csv-file/
     
    // the input is the file: "fileToOpen.csv":
    // a,b,c
    // d,e,f
    // This function converts input file data into the Eigen matrix format 
    // The matrix entries are stored in this variable row-wise. For example if we have the matrix:
    // M=[a b c 
    //    d e f]
    // the entries are stored as matrixEntries=[a,b,c,d,e,f], that is the variable "matrixEntries" is a row vector
    // later on, this vector is mapped into the Eigen matrix format
    std::vector<double> matrixEntries;
 
    // in this object we store the data from the matrix
    std::ifstream matrixDataFile(fileToOpen);
 
    // this variable is used to store the row of the matrix that contains commas 
    std::string matrixRowString;
 
    // this variable is used to store the matrix entry;
    std::string matrixEntry;
 
    // this variable is used to track the number of rows
    int matrixRowNumber = 0;
 
 
    while (getline(matrixDataFile, matrixRowString)) // here we read a row by row of matrixDataFile and store every line into the string variable matrixRowString
    {
        std::stringstream matrixRowStringStream(matrixRowString); //convert matrixRowString that is a string to a stream variable.
 
        while (getline(matrixRowStringStream, matrixEntry, ',')) // here we read pieces of the stream matrixRowStringStream until every comma, and store the resulting character into the matrixEntry
        {
            matrixEntries.push_back(stod(matrixEntry));   //here we convert the string to double and fill in the row vector storing all the matrix entries
        }
        matrixRowNumber++; //update the column numbers
    }
 
    // here we convet the vector variable into the matrix and return the resulting object, 
    // note that matrixEntries.data() is the pointer to the first memory location at which the entries of the vector matrixEntries are stored;
    return Eigen::Map<Eigen::Matrix<double, Eigen::Dynamic, Eigen::Dynamic, Eigen::RowMajor>>(matrixEntries.data(), matrixRowNumber, matrixEntries.size() / matrixRowNumber);
 
}

// Save matrix in CSV format
void eigen2csv(std::string fileName, Eigen::MatrixXd  matrix)
{
    // Copy&Paste from https://aleksandarhaber.com/eigen-matrix-library-c-tutorial-saving-and-loading-data-in-from-a-csv-file/

    //https://eigen.tuxfamily.org/dox/structEigen_1_1IOFormat.html
    const static Eigen::IOFormat CSVFormat(Eigen::FullPrecision, Eigen::DontAlignCols, ", ", "\n");
 
    std::ofstream file(fileName);
    if (file.is_open())
    {
        file << matrix.format(CSVFormat);
        file.close();
    }
}

// Save list (std::vector) of eigen vectors row-wise in CSV format
void eigen2csv(std::string fileName, std::vector<Eigen::VectorXd> data)
{
    const static Eigen::IOFormat CSVFormat(Eigen::FullPrecision, Eigen::DontAlignCols, ", ", "\n");
    
    std::ofstream file(fileName);
    if(file.is_open())
    {
        for(const auto& v: data)
        {
            file << v.transpose().format(CSVFormat) << '\n';
        }
        file.close();
    }
}