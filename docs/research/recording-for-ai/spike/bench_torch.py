# PyTorch side of the comparison: same network, data, batch order and optimizer
# as rust/src/main.rs. Prints one JSON line per seed.
import json, sys, time
import numpy as np
import torch
from torch import nn

N, SIZE, BATCH, STEPS, WARMUP = 1024, 128, 64, 1000, 50
started = time.perf_counter()
device = torch.device(sys.argv[1] if len(sys.argv) > 1 else 'cuda')
images = torch.from_numpy(np.fromfile('images.bin', dtype=np.float32).reshape(N, 3, SIZE, SIZE)).to(device)
targets = torch.from_numpy(np.fromfile('targets.bin', dtype=np.float32).reshape(N, 2)).to(device)


def net():
    layers, width = [], 3
    for out in (32, 64, 128):
        layers += [nn.Conv2d(width, out, 3, padding=1), nn.ReLU(), nn.MaxPool2d(2)]
        width = out
    layers += [nn.Conv2d(128, 256, 3, padding=1), nn.ReLU(), nn.AdaptiveAvgPool2d(1), nn.Flatten(), nn.Linear(256, 2)]
    return nn.Sequential(*layers).to(device)


def batch(step):
    index = torch.tensor([(step * BATCH + i) % N for i in range(BATCH)], device=device)
    return images[index], targets[index]


for seed in (1, 2, 3):
    torch.manual_seed(seed)
    model = net()
    optimizer = torch.optim.Adam(model.parameters(), lr=1e-3)
    if device.type == 'cuda':
        torch.cuda.reset_peak_memory_stats()
    timed_from, losses = None, []
    for step in range(STEPS):
        if step == WARMUP:
            timed_from = time.perf_counter()
        x, y = batch(step)
        loss = ((model(x) - y) ** 2).mean()
        optimizer.zero_grad()
        loss.backward()
        optimizer.step()
        losses.append(loss.item())
    took = time.perf_counter() - timed_from
    print(json.dumps({
        'framework': 'pytorch ' + torch.__version__, 'device': str(device), 'seed': seed,
        'steps_per_s': round((STEPS - WARMUP) / took, 1),
        'final_mse': round(float(np.mean(losses[-50:])), 6),
        'peak_mb': round(torch.cuda.max_memory_allocated() / 1e6) if device.type == 'cuda' else None,
        'since_start_s': round(time.perf_counter() - started, 1),
    }), flush=True)
