import numpy as np
from numpy.linalg import inv

class KalmanFilter:
    def __init__(self, x_init, P_init, F, G, H, D, R):
        self.x = x_init
        self.dim = len(self.x)
        self.P = P_init
        self.F = F
        self.G = G
        self.H = H
        self.D = D
        self.R = R

    def filter(self, measure):
        et = measure-self.H@self.x
        St = self.H@self.x@self.H.transpose()+self.R
        K = self.P@self.H.transpose()@inv(St)
        self.x = self.x+K@et
        self.P = (np.eye(self.dim)-K@self.H)@self.P

    def predict(self, control_input=None):
        self.x = self.F @ self.x
        if input is not None:
            self.x += self.B@control_input
        self.P = self.F@self.P@self.F.transpose() + self.Q
