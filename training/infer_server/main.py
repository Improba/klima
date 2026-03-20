"""PyTorch FNO inference sidecar for Klima (trained checkpoint, no ONNX FFT).

Wire format POST /predict: body = magic (u32 LE) + ndim (u32 LE) + shape (ndim × u32 LE)
+ raw float32 row-major (C) data. Response uses the same header + output tensor.
"""

from __future__ import annotations

import json
import os
import struct
from contextlib import asynccontextmanager
from pathlib import Path
from typing import Any, Dict, Optional

import numpy as np
import torch
from fastapi import FastAPI, Request, Response
from fastapi.responses import PlainTextResponse

MAGIC = int.from_bytes(b"KLM1", "little")

_state: Dict[str, Any] = {}


def _load_norm(path: Path) -> Optional[Dict[str, np.ndarray]]:
    if not path.is_file():
        return None
    with open(path) as f:
        raw = json.load(f)
    return {
        "input_mean": np.asarray(raw["input_mean"], dtype=np.float32),
        "input_std": np.maximum(np.asarray(raw["input_std"], dtype=np.float32), 1e-8),
        "output_mean": np.asarray(raw["output_mean"], dtype=np.float32),
        "output_std": np.asarray(raw["output_std"], dtype=np.float32),
    }


def _normalize(x: torch.Tensor, norm: Dict[str, np.ndarray]) -> torch.Tensor:
    dev = x.device
    mean = torch.from_numpy(norm["input_mean"]).to(dev).view(1, -1, 1, 1, 1)
    std = torch.from_numpy(norm["input_std"]).to(dev).view(1, -1, 1, 1, 1)
    return (x - mean) / std


def _denormalize(y: torch.Tensor, norm: Dict[str, np.ndarray]) -> torch.Tensor:
    dev = y.device
    mean = torch.from_numpy(norm["output_mean"]).to(dev).view(1, -1, 1, 1, 1)
    std = torch.from_numpy(norm["output_std"]).to(dev).view(1, -1, 1, 1, 1)
    return y * std + mean


def _decode_request(body: bytes) -> np.ndarray:
    if len(body) < 8:
        raise ValueError("body too short")
    magic, ndim = struct.unpack_from("<II", body, 0)
    if magic != MAGIC:
        raise ValueError(f"bad magic {magic:#x}")
    if ndim < 1 or ndim > 8:
        raise ValueError(f"bad ndim {ndim}")
    need = 8 + 4 * ndim
    if len(body) < need:
        raise ValueError("truncated shape")
    shape = struct.unpack_from(f"<{ndim}I", body, 8)
    data = body[need:]
    n_float = len(data) // 4
    expected = int(np.prod(shape, dtype=np.int64))
    if n_float != expected:
        raise ValueError(f"size mismatch: got {n_float} floats, need {expected}")
    return np.frombuffer(data, dtype=np.float32, count=expected).reshape(shape)


def _encode_response(arr: np.ndarray) -> bytes:
    shape = arr.shape
    ndim = len(shape)
    header = struct.pack("<II", MAGIC, ndim)
    header += struct.pack(f"<{ndim}I", *shape)
    return header + arr.astype(np.float32, copy=False).tobytes()


@asynccontextmanager
async def lifespan(app: FastAPI):
    ckpt_path = os.environ.get("KLIMA_FNO_CHECKPOINT", "/checkpoints/best_model.pt")
    norm_path = Path(os.environ.get("KLIMA_FNO_NORM", "/checkpoints/norm_params.json"))

    ckpt_file = Path(ckpt_path)
    if not ckpt_file.is_file():
        print(f"[infer] No checkpoint at {ckpt_path} — /predict will return 503")
        _state["model"] = None
        _state["norm"] = None
        _state["device"] = torch.device("cpu")
        yield
        return

    # Import after PYTHONPATH includes repo root (see docker-compose)
    from training.src.model.local_fno import LocalFNO3d

    ckpt = torch.load(ckpt_path, map_location="cpu", weights_only=False)
    cfg = ckpt["config"]["model"]
    model = LocalFNO3d(
        in_channels=cfg["in_channels"],
        out_channels=cfg["out_channels"],
        modes=tuple(cfg["modes"]),
        width=cfg["width"],
        num_layers=cfg["num_layers"],
    )
    model.load_state_dict(ckpt["model_state_dict"])
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    model.to(device)
    model.eval()
    _state["model"] = model
    _state["device"] = device
    _state["norm"] = _load_norm(norm_path)
    if _state["norm"] is None:
        print(f"[infer] Warning: no norm params at {norm_path}")

    print(f"[infer] Loaded FNO from {ckpt_path} on {device}")
    yield


app = FastAPI(title="Klima FNO infer", lifespan=lifespan)


@app.get("/health")
def health():
    ok = _state.get("model") is not None
    return {"status": "ok" if ok else "no_model", "device": str(_state.get("device", ""))}


@app.post("/predict")
async def predict(request: Request) -> Response:
    model = _state.get("model")
    if model is None:
        return PlainTextResponse("model not loaded", status_code=503)

    body = await request.body()
    try:
        arr = _decode_request(body)
    except ValueError as e:
        return PlainTextResponse(str(e), status_code=400)

    device = _state["device"]
    norm = _state["norm"]

    x = torch.from_numpy(arr.copy()).to(device)
    with torch.no_grad():
        if norm is not None:
            x = _normalize(x, norm)
        y = model(x)
        if norm is not None:
            y = _denormalize(y, norm)
    out_np = y.detach().cpu().numpy().astype(np.float32)
    payload = _encode_response(out_np)
    return Response(content=payload, media_type="application/octet-stream")
