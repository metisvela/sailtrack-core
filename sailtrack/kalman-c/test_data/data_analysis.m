true_x = readmatrix('ground_truth.csv');
noisy_x = readmatrix('output.csv');
estimate_x = (readmatrix('estimates.csv'))'; %NOTE THE TRANSPOSE

% Plots for the curious
t = 1:1000;

subplot(2,3,1)
plot(t, true_x(1,:))
subtitle('True')
ylabel('pos_x')
axis([990 1000 -1 1])

subplot(2, 3, 2)
plot(t, noisy_x(1,:))
subtitle('Measurements')
axis([990 1000 -1 1])

subplot(2,3,3)
plot(t, estimate_x(1,:))
subtitle('Estimates')
axis([990 1000 -1 1])

subplot(2,3,4)
plot(t, true_x(4,:))
ylabel('vel_x')
axis([990 1000 -1 1])

subplot(2, 3, 5)
plot(t, noisy_x(4,:))
axis([990 1000 -1 1])

subplot(2, 3, 6)
plot(t, estimate_x(4,:))
axis([990 1000 -1 1])
