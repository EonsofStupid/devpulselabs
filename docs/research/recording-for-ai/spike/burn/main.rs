// Burn side of the comparison: same network, data, batch order and optimizer as
// bench_torch.py. Prints one JSON line per seed. Run from the folder holding
// images.bin and targets.bin.
use burn::backend::{wgpu::WgpuDevice, Autodiff, Vulkan};
use burn::module::Module;
use burn::nn::conv::{Conv2d, Conv2dConfig};
use burn::nn::pool::{AdaptiveAvgPool2d, AdaptiveAvgPool2dConfig, MaxPool2d, MaxPool2dConfig};
use burn::nn::{Linear, LinearConfig, PaddingConfig2d, Relu};
use burn::optim::{AdamConfig, GradientsParams, Optimizer};
use burn::tensor::backend::{AutodiffBackend, Backend};
use burn::tensor::{Tensor, TensorData};
use std::time::Instant;

const N: usize = 1024;
const SIZE: usize = 128;
const BATCH: usize = 64;
const STEPS: usize = 1000;
const WARMUP: usize = 50;

#[derive(Module, Debug)]
struct Net<B: Backend> {
    conv1: Conv2d<B>,
    conv2: Conv2d<B>,
    conv3: Conv2d<B>,
    conv4: Conv2d<B>,
    pool: MaxPool2d,
    average: AdaptiveAvgPool2d,
    out: Linear<B>,
    relu: Relu,
}

impl<B: Backend> Net<B> {
    fn new(device: &B::Device) -> Self {
        let conv = |from, to| {
            Conv2dConfig::new([from, to], [3, 3])
                .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
                .init(device)
        };
        Self {
            conv1: conv(3, 32),
            conv2: conv(32, 64),
            conv3: conv(64, 128),
            conv4: conv(128, 256),
            pool: MaxPool2dConfig::new([2, 2]).with_strides([2, 2]).init(),
            average: AdaptiveAvgPool2dConfig::new([1, 1]).init(),
            out: LinearConfig::new(256, 2).init(device),
            relu: Relu::new(),
        }
    }

    fn forward(&self, x: Tensor<B, 4>) -> Tensor<B, 2> {
        let x = self.pool.forward(self.relu.forward(self.conv1.forward(x)));
        let x = self.pool.forward(self.relu.forward(self.conv2.forward(x)));
        let x = self.pool.forward(self.relu.forward(self.conv3.forward(x)));
        let x = self.average.forward(self.relu.forward(self.conv4.forward(x)));
        self.out.forward(x.flatten(1, 3))
    }
}

fn floats(path: &str) -> Vec<f32> {
    std::fs::read(path)
        .expect(path)
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect()
}

fn run<B: AutodiffBackend<FloatElem = f32>>(device: &B::Device, seed: u64, started: Instant) {
    B::seed(device, seed);
    let image_data = floats("images.bin");
    let target_data = floats("targets.bin");
    let images: Tensor<B, 4> =
        Tensor::from_data(TensorData::new(image_data, [N, 3, SIZE, SIZE]), device);
    let targets: Tensor<B, 2> = Tensor::from_data(TensorData::new(target_data, [N, 2]), device);
    let mut model = Net::<B>::new(device);
    let mut optimizer = AdamConfig::new().init();
    let (mut timed_from, mut losses) = (Instant::now(), Vec::with_capacity(STEPS));
    for step in 0..STEPS {
        if step == WARMUP {
            timed_from = Instant::now();
        }
        let index: Vec<i64> = (0..BATCH).map(|i| ((step * BATCH + i) % N) as i64).collect();
        let index = Tensor::<B, 1, burn::tensor::Int>::from_data(
            TensorData::new(index, [BATCH]),
            device,
        );
        let x = images.clone().select(0, index.clone());
        let y = targets.clone().select(0, index);
        let difference = model.forward(x) - y;
        let loss = (difference.clone() * difference).mean();
        losses.push(loss.clone().into_scalar());
        let gradients = GradientsParams::from_grads(loss.backward(), &model);
        model = optimizer.step(1e-3, model, gradients);
        if step < 5 || step % 25 == 0 {
            eprintln!(
                "seed {seed} step {step} at {:.1}s loss {:.5}",
                started.elapsed().as_secs_f64(),
                losses[step]
            );
        }
    }
    let took = timed_from.elapsed().as_secs_f64();
    let tail = &losses[STEPS - 50..];
    println!(
        "{{\"framework\":\"burn 0.21 vulkan\",\"seed\":{seed},\"steps_per_s\":{:.1},\"final_mse\":{:.6},\"since_start_s\":{:.1}}}",
        (STEPS - WARMUP) as f64 / took,
        tail.iter().sum::<f32>() / tail.len() as f32,
        started.elapsed().as_secs_f64()
    );
}

fn main() {
    let started = Instant::now();
    // The RTX 5090, not the integrated Intel GPU.
    let device = WgpuDevice::DiscreteGpu(0);
    for seed in [1, 2, 3] {
        run::<Autodiff<Vulkan<f32, i32>>>(&device, seed, started);
    }
}
