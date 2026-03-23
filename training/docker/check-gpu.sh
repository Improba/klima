#!/usr/bin/env bash
# Vérifie l’environnement d’entraînement dans Docker (PyTorch + GPU si présent).
# Rien à installer sur l’hôte sauf Docker ; torch est dans l’image.
set -euo pipefail
cd "$(dirname "$0")"
echo "=== Smoke test PyTorch + loss PINN (service klima-training-verify, GPU non requis) ==="
docker compose run -T --rm klima-training-verify
echo ""
echo "=== nvidia-smi (conteneur klima-training, requiert NVIDIA Container Toolkit + GPU) ==="
docker compose run --rm klima-training nvidia-smi
echo ""
echo "=== PyTorch CUDA (conteneur klima-training) ==="
docker compose run -T --rm klima-training python -c \
  "import torch; print('torch.cuda.is_available():', torch.cuda.is_available()); print('device:', torch.cuda.get_device_name(0) if torch.cuda.is_available() else None)"
