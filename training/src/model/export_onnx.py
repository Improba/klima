"""Export the trained Local-FNO model to ONNX format.

Dynamic axes on batch size and all three spatial dimensions ensure the
exported model accepts variable-resolution inputs at inference time.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Dict, Optional

import torch

from .local_fno import LocalFNO3d


def export_to_onnx(
    model: LocalFNO3d,
    output_path: str,
    norm_params_path: Optional[str] = None,
    norm_params: Optional[Dict[str, Any]] = None,
    opset_version: int = 17,
) -> None:
    """Export the FNO to ONNX with dynamic axes for batch and spatial dims.

    Parameters
    ----------
    model : LocalFNO3d
        Trained model (will be set to eval mode).
    output_path : str
        Path for the ``.onnx`` file.
    norm_params_path : str, optional
        If given, save normalisation parameters (mu, sigma per channel) as JSON.
    norm_params : dict, optional
        Dict with keys ``input_mean``, ``input_std``, ``output_mean``, ``output_std``.
    opset_version : int
        ONNX opset version.
    """
    model.eval()
    device = next(model.parameters()).device

    dummy_input = torch.randn(1, model.in_channels, 32, 32, 16, device=device)

    dynamic_axes = {
        "input": {0: "batch_size", 2: "nx", 3: "ny", 4: "nz"},
        "output": {0: "batch_size", 2: "nx", 3: "ny", 4: "nz"},
    }

    torch.onnx.export(
        model,
        dummy_input,
        output_path,
        opset_version=opset_version,
        input_names=["input"],
        output_names=["output"],
        dynamic_axes=dynamic_axes,
    )

    if norm_params_path and norm_params:
        serialisable = {}
        for k, v in norm_params.items():
            if isinstance(v, torch.Tensor):
                serialisable[k] = v.tolist()
            elif hasattr(v, "tolist"):
                serialisable[k] = v.tolist()
            else:
                serialisable[k] = v
        with open(norm_params_path, "w") as f:
            json.dump(serialisable, f, indent=2)


def main() -> None:
    parser = argparse.ArgumentParser(description="Export Klima FNO to ONNX")
    parser.add_argument("--checkpoint", type=str, required=True, help="Path to .pt checkpoint")
    parser.add_argument("--output", type=str, default="model.onnx")
    parser.add_argument("--norm-output", type=str, default="norm_params.json")
    args = parser.parse_args()

    ckpt = torch.load(args.checkpoint, map_location="cpu", weights_only=False)
    cfg = ckpt["config"]["model"]

    model = LocalFNO3d(
        in_channels=cfg["in_channels"],
        out_channels=cfg["out_channels"],
        modes=tuple(cfg["modes"]),
        width=cfg["width"],
        num_layers=cfg["num_layers"],
    )
    model.load_state_dict(ckpt["model_state_dict"])

    norm_path = Path(args.checkpoint).parent / "norm_params.json"
    norm_params = None
    if norm_path.exists():
        with open(norm_path) as f:
            norm_params = json.load(f)

    export_to_onnx(model, args.output, args.norm_output, norm_params)
    print(f"Exported ONNX model to {args.output}")


if __name__ == "__main__":
    main()
