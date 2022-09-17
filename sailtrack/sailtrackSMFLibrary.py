import numpy as np
from numpy.linalg import inv


class KalmanFilter:
    def __init__(self, x_init, P_init=None, F=None, G=None, H=None, D=None, Q=None, S=None, R=None):
        self.x = x_init
        self.dim = len(self.x)
        if P_init is None:
            self.P = np.eye(self.dim)
        else:
            self.P = P_init
        self.F = F
        self.G = G
        self.H = H
        self.D = D
        self.Q = Q
        self.S = S
        self.R = R

    def filter(self, measure):
        innovation = measure - self.H @ self.x
        delta = self.H @ self.P @ self.H.transpose() + self.R
        k_gain = self.P @ self.H.transpose() @ inv(delta)
        self.x = self.x + k_gain @ innovation
        self.P = self.P - k_gain @ self.H @ self.P

    def predict(self, control_input=None, measure=None):
        self.x = self.F @ self.x
        if input is not None:
            self.x += self.G @ control_input
        if self.S is not None:
            self.x += self.S @ inv(self.R) @ measure
        self.P = self.F @ self.P @ self.F.transpose() + self.Q

