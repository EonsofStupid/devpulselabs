#!/bin/sh
# A GPU Python for the training comparison, separate from the global Python
# (whose torch is CPU-only and may be used by other projects). Scratch only.
PATH="/usr/bin:$PATH"; export PATH
cd "$(dirname "$0")" || exit 1
python -m venv .venv || exit 1
PY=.venv/Scripts/python.exe
"$PY" -m pip install --upgrade pip wheel >/dev/null
# Blackwell needs CUDA 12.8 or newer; take the newest wheel index that has torch.
for index in cu130 cu129 cu128; do
  echo "trying $index"
  if "$PY" -m pip install torch torchvision --index-url "https://download.pytorch.org/whl/$index"; then
    echo "torch from $index"
    break
  fi
done
"$PY" -m pip install numpy pillow transformers peft trl accelerate datasets bitsandbytes unsloth qwen-vl-utils
"$PY" -c "import torch; print('torch', torch.__version__, 'cuda', torch.version.cuda, 'gpu', torch.cuda.is_available(), torch.cuda.get_device_name(0) if torch.cuda.is_available() else '')"
"$PY" -m pip list 2>/dev/null | grep -Ei "^(torch|transformers|peft|trl|unsloth|accelerate|bitsandbytes) "
du -sh .venv
