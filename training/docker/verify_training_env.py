#!/usr/bin/env python3
"""
Vérification PyTorch + imports Klima dans l’image d’entraînement.

Usage (depuis la racine du monorepo, via Compose) :
  cd training/docker && docker compose run --rm klima-training-verify

Ne pas exécuter sur l’hôte sans environnement torch : tout passe par Docker.
"""
from __future__ import annotations

import sys


def main() -> int:
    try:
        import torch
    except ImportError as e:
        print("verify_training_env: torch introuvable — utilisez l’image Docker.", file=sys.stderr)
        print(e, file=sys.stderr)
        return 1

    print(f"torch {torch.__version__}, cuda_available={torch.cuda.is_available()}")

    from training.src.model.loss import PINNLoss, impermeability_loss

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    B, X, Y, Z = 1, 8, 8, 8
    pred = torch.zeros(B, 4, X, Y, Z, device=device)
    tgt = torch.zeros_like(pred)
    mask_air = torch.ones(B, 1, X, Y, Z, device=device)
    mask_solid = 1.0 - mask_air
    mask_surface = torch.zeros(B, 1, X, Y, Z, device=device)
    alpha = torch.ones(B, 1, X, Y, Z, device=device)

    criterion = PINNLoss(lambda_impermeability=0.1, dx=2.0)
    losses = criterion(pred, tgt, mask_air, mask_solid, mask_surface, alpha)
    assert "imperm" in losses and "total" in losses

    vx = torch.randn(B, 1, X, Y, Z, device=device)
    vy = torch.randn(B, 1, X, Y, Z, device=device)
    vz = torch.randn(B, 1, X, Y, Z, device=device)
    _ = impermeability_loss(vx, vy, vz, mask_air, mask_solid)

    print("verify_training_env: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
