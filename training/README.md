# Klima — Training Pipeline

Training pipeline for the urban microclimate 3D simulator using a Local Fourier Neural Operator (FNO).

## Quick Start (local Python)

From the **monorepo root** (`klima/`), so that the package `training.src` resolves:

```bash
pip install -r training/requirements.txt
PYTHONPATH=. python -m training.src.model.train --config training/configs/default.yaml
```

## Docker

Requires an NVIDIA GPU and the [NVIDIA Container Toolkit](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/install-guide.html).

- Compose **project name** `klima-training` (separate from the dev stack `name: klima` in `back/` / `front/`).
- The repo root is mounted at `/app`, `PYTHONPATH=/app`, and **`working_dir`** is `/app/training` so paths in `configs/default.yaml` (`data/…`, `checkpoints/`, etc.) stay correct.
- The service sets `runtime: nvidia`, GPU device reservations, and `NVIDIA_DRIVER_CAPABILITIES`. If the daemon reports **unknown runtime: nvidia**, remove the `runtime: nvidia` line in `docker/docker-compose.yml` and rely on `deploy.resources` only.

```bash
cd training/docker && docker compose up --build
```

## Mock ONNX (for backend dev)

From the monorepo root:

```bash
PYTHONPATH=. python -m training.src.model.mock_onnx --output model.onnx --norm-output norm_params.json
```
