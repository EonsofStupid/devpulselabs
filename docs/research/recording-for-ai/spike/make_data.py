# The same data for both frameworks, as raw little-endian floats: noisy frames
# with one bright rectangle, labelled with its centre (a stand-in for "where is
# the control"). Written once, so PyTorch and Burn train on identical bytes.
import numpy as np

N, SIZE = 1024, 128
rng = np.random.default_rng(7)
images = rng.random((N, 3, SIZE, SIZE), dtype=np.float32) * 0.25
targets = np.zeros((N, 2), dtype=np.float32)
for i in range(N):
    w, h = rng.integers(16, 40, size=2)
    x, y = rng.integers(0, SIZE - w), rng.integers(0, SIZE - h)
    images[i, :, y:y + h, x:x + w] = (rng.random(3, dtype=np.float32) * 0.3 + 0.7).reshape(3, 1, 1)
    targets[i] = ((x + w / 2) / SIZE, (y + h / 2) / SIZE)
images.tofile('images.bin')
targets.tofile('targets.bin')
print('images', images.shape, 'targets', targets.shape, 'first target', targets[0])
