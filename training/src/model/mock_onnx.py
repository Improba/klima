"""Generate a lightweight mock ONNX model for backend development.

The mock model has the correct I/O signature (15-channel input, 4-channel
output, variable spatial dims) but uses trivial Conv3d layers instead of a
full FNO, enabling fast inference during integration testing.

Usage::

    python -m training.src.model.mock_onnx --output model.onnx --norm-output norm_params.json
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Dict, List

import torch
import torch.nn as nn


class _MockFNO(nn.Module):
    """Minimal Conv3d stand-in with matching I/O dimensions."""

    def __init__(self, in_channels: int = 15, out_channels: int = 4) -> None:
        super().__init__()
        self.net = nn.Sequential(
            nn.Conv3d(in_channels, 32, kernel_size=3, padding=1),
            nn.ReLU(),
            nn.Conv3d(32, 32, kernel_size=3, padding=1),
            nn.ReLU(),
            nn.Conv3d(32, out_channels, kernel_size=1),
        )

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return self.net(x)


def create_mock_onnx(
    output_path: str,
    norm_path: str,
    in_channels: int = 15,
    out_channels: int = 4,
    opset_version: int = 17,
) -> None:
    """Create a lightweight mock ONNX model with correct I/O dimensions.

    Parameters
    ----------
    output_path : str
        Path for the ``.onnx`` file.
    norm_path : str
        Path for mock normalisation parameters JSON.
    in_channels : int
        Number of input channels (default 15).
    out_channels : int
        Number of output channels (default 4).
    opset_version : int
        ONNX opset version.
    """
    model = _MockFNO(in_channels, out_channels)
    model.eval()

    dummy = torch.randn(1, in_channels, 32, 32, 16)

    dynamic_axes = {
        "input": {0: "batch_size", 2: "nx", 3: "ny", 4: "nz"},
        "output": {0: "batch_size", 2: "nx", 3: "ny", 4: "nz"},
    }

    torch.onnx.export(
        model,
        dummy,
        output_path,
        opset_version=opset_version,
        input_names=["input"],
        output_names=["output"],
        dynamic_axes=dynamic_axes,
    )

    norm_params: Dict[str, List[float]] = {
        "input_mean": [0.0] * in_channels,
        "input_std": [1.0] * in_channels,
        "output_mean": [0.0] * out_channels,
        "output_std": [1.0] * out_channels,
    }
    with open(norm_path, "w") as f:
        json.dump(norm_params, f, indent=2)

    print(f"Mock ONNX model saved to {output_path}")
    print(f"Mock norm params saved to {norm_path}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Generate mock ONNX for Klima backend dev")
    parser.add_argument("--output", type=str, default="model.onnx", help="ONNX output path")
    parser.add_argument("--norm-output", type=str, default="norm_params.json", help="Norm params path")
    args = parser.parse_args()
    create_mock_onnx(args.output, args.norm_output)


if __name__ == "__main__":
    main()
