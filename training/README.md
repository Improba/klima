# Klima — Training Pipeline

Training pipeline for the urban microclimate 3D simulator using a Local Fourier Neural Operator (FNO).

## Quick Start

```bash
pip install -r requirements.txt
python -m training.src.model.train --config training/configs/default.yaml
```

## Docker

```bash
cd training/docker && docker compose up --build
```

## Mock ONNX (for backend dev)

```bash
python -m training.src.model.mock_onnx --output model.onnx --norm-output norm_params.json
```
