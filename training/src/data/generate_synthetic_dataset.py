"""Generate HDF5 training data from physics-structured synthetic fields.

See :mod:`training.src.data.synthetic_physics` for the governing assumptions.

Example (from monorepo root)::

    PYTHONPATH=. python -m training.src.data.generate_synthetic_dataset \\
        --output training/data/synthetic_cfd_local.h5 \\
        --nx 72 --ny 72 --nz 24 --n-train 24 --n-val 6 --seed 42
"""

from __future__ import annotations

import argparse
from pathlib import Path

import h5py
import numpy as np
import yaml

from training.src.data.synthetic_physics import synthesize_sample


def _write_split(
    f: h5py.File,
    split: str,
    n: int,
    nx: int,
    ny: int,
    nz: int,
    dx: float,
    rng: np.random.Generator,
) -> list:
    """Write groups ``{split}/0`` ... and return list of sample dicts for stats."""
    samples = []
    for i in range(n):
        d = synthesize_sample(nx, ny, nz, dx, rng)
        samples.append(d)
        g = f.create_group(f"{split}/{i}")
        g.create_dataset("input", data=d["input"], compression="gzip", compression_opts=4)
        g.create_dataset("output", data=d["output"], compression="gzip", compression_opts=4)
        g.create_dataset("mask_air", data=d["mask_air"], compression="gzip", compression_opts=4)
        g.create_dataset("mask_solid", data=d["mask_solid"], compression="gzip", compression_opts=4)
        g.create_dataset("mask_surface", data=d["mask_surface"], compression="gzip", compression_opts=4)
        g.create_dataset("alpha_field", data=d["alpha_field"], compression="gzip", compression_opts=4)
    return samples


def _stack_stats(samples: list) -> tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray]:
    inp = np.stack([s["input"] for s in samples], axis=0)
    out = np.stack([s["output"] for s in samples], axis=0)
    # mean/std over batch and spatial dims
    input_mean = inp.mean(axis=(0, 2, 3, 4))
    input_std = inp.std(axis=(0, 2, 3, 4))
    input_std = np.maximum(input_std, 1e-6)
    output_mean = out.mean(axis=(0, 2, 3, 4))
    output_std = out.std(axis=(0, 2, 3, 4))
    output_std = np.maximum(output_std, 1e-6)
    return input_mean, input_std, output_mean, output_std


def main() -> None:
    p = argparse.ArgumentParser(description="Generate synthetic CFD-style HDF5 for Klima training")
    p.add_argument("--output", type=str, required=True, help="Output .h5 path")
    p.add_argument("--nx", type=int, default=72)
    p.add_argument("--ny", type=int, default=72)
    p.add_argument("--nz", type=int, default=24)
    p.add_argument("--dx", type=float, default=2.0)
    p.add_argument("--n-train", type=int, default=24)
    p.add_argument("--n-val", type=int, default=6)
    p.add_argument("--seed", type=int, default=42)
    p.add_argument(
        "--meta-yaml",
        type=str,
        default=None,
        help="Optional: copy domain nx,ny,nz,dx from this training config YAML",
    )
    args = p.parse_args()

    nx, ny, nz, dx = args.nx, args.ny, args.nz, args.dx
    if args.meta_yaml:
        with open(args.meta_yaml) as fh:
            cfg = yaml.safe_load(fh)
        dom = cfg.get("domain", {})
        nx = int(dom.get("nx", nx))
        ny = int(dom.get("ny", ny))
        nz = int(dom.get("nz", nz))
        dx = float(dom.get("dx", dx))

    out_path = Path(args.output)
    out_path.parent.mkdir(parents=True, exist_ok=True)

    rng = np.random.default_rng(args.seed)
    train_rng = np.random.default_rng(args.seed)
    val_rng = np.random.default_rng(args.seed + 10_000)

    with h5py.File(out_path, "w") as f:
        train_samples = _write_split(
            f, "train", args.n_train, nx, ny, nz, dx, train_rng
        )
        _write_split(f, "val", args.n_val, nx, ny, nz, dx, val_rng)

        im, istd, om, ostd = _stack_stats(train_samples)
        f.attrs["input_mean"] = im
        f.attrs["input_std"] = istd
        f.attrs["output_mean"] = om
        f.attrs["output_std"] = ostd
        f.attrs["generator"] = "training.src.data.synthetic_physics"
        f.attrs["description"] = (
            "Laplace T in air + div-free wind (curl B); see synthetic_physics module docstring."
        )
        f.attrs["nx"] = nx
        f.attrs["ny"] = ny
        f.attrs["nz"] = nz
        f.attrs["dx"] = dx

    print(f"Wrote {out_path} (train={args.n_train}, val={args.n_val}, grid={nx}x{ny}x{nz}).")


if __name__ == "__main__":
    main()
