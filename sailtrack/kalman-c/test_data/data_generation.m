% Generate Test data (1D) for Kalman-Filter:
% Generation: a(t) modeled as unit square wave, x(t),v(t) computed
%             analytically from that at sampled time. (dt = 1)

% Create acceleration data
a = zeros(1, 1000);
a(1) = 1;
for i = 2:4:994
    a(i) = -1;
    a(i+1) = -1;
    a(i+2) = 1;
    a(i+3) = 1;
end
a(998) = -1;
a(999) = -1;
a(1000) = 1;

% Velocity data (evaluated at sampled time after analytical integration)
v = zeros(1, 1000);
for i = 1:4:1000
    v(i) = 0.5;
    v(i+1) = 0.5;
    v(i+2) = -0.5;
    v(i+3) = -0.5;
end

% Position data
x = zeros(1, 1000);
for i = 1:4:1000
    x(i) = 1/8;
    x(i+1) = 7/8;
    x(i+2) = 7/8;
    x(i+3) = 1/8;
end

% Noise (first add only in 1D)
w_std = 0.1;
r_std = 0.05;

a_noise = normrnd(0, w_std, size(a));
v_noise = normrnd(0, r_std, size(v));
x_noise = normrnd(0, r_std, size(x));

% Vectorize (assume all movement in x-direction)
input_vectors = [a + a_noise;
                 normrnd(0, w_std, size(a));
                 normrnd(0, w_std, size(a))];

measurment_vectors = [x + x_noise;
                      normrnd(0, r_std, size(a));
                      normrnd(0, r_std, size(a));
                      v + v_noise;
                      normrnd(0, r_std, size(a));
                      normrnd(0, r_std, size(a))];

ground_truth = [x;
                zeros(size(a));
                zeros(size(a));
                v;
                zeros(size(a));
                zeros(size(a))];

% plot acceleration for sanity test
t = 1:1000;
subplot(3,1,1)
plot(t, input_vectors(1,:))
axis([990 1000 -1 1])
subplot(3,1,2)
plot(t, input_vectors(2,:))
axis([990 1000 -1 1])
subplot(3,1,3)
plot(t, input_vectors(3,:))
axis([990 1000 -1 1])




writematrix(input_vectors, 'input.csv');
writematrix(measurment_vectors, 'output.csv');
writematrix(ground_truth, 'ground_truth.csv')