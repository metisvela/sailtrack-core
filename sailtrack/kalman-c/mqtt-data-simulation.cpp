#include <iostream>
#include <fstream>
#include <sstream>
#include <vector>

#define CSV_NAME    "sensor-data-2022-09-07.csv"
#define NUM_DATA    22   //columns in the .csv file

using namespace std;

int main() {
    vector<vector<string>> data; 

    ifstream file(CSV_NAME); 
    string line;

    while (getline(file, line)) {
        vector<string> row; 
        stringstream ss(line);
        string value;
        int count = 0;
        while (getline(ss, value, ',')) {
            if(value == ""){
                break;
            }
            row.push_back(value); 
            count++;
        }
        while(count < NUM_DATA){        
            row.push_back("NULL");         
            count++;                    
        } 
        data.push_back(row); 
    }
    file.close(); 
    for (int i = 0; i < data.size(); i++) {
        for (int j = 0; j < data[i].size(); j++) {
            cout << data[i][j] << " ";
        }
        cout << endl;
    }
    return 0;
}
