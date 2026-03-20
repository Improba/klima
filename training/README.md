# Klima — Training Pipeline

Training pipeline for the urban microclimate 3D simulator using a Local Fourier Neural Operator (FNO).

## Quick Start (local Python)

From the **monorepo root** (`klima/`), so that the package `training.src` resolves:

```bash
pip install -r training/requirements.txt
PYTHONPATH=. python -m training.src.model.train --config training/configs/default.yaml
```

`default.yaml` expects a real CFD dataset at `training/data/cfd_dataset.h5`. For **local experiments without OpenFOAM**, use the synthetic pipeline below.

## Données synthétiques (physique explicite, hors CFD)

Le module `training/src/data/synthetic_physics.py` construit des paires entrée/sortie compatibles avec le chargeur HDF5 :

- **Température** : Laplace ∇²*T* = 0 dans l’air, températures imposées dans les solides et sur le pourtour du domaine (conductif stationnaire simplifié).
- **Vent** : champ **à divergence nulle** *v* = ∇×**B** (**B** = bruit gaussien lissé) + vent moyen horizontal selon la direction méto.

Ce n’est **pas** un substitut à un maillage CFD urbain, mais une base reproductible pour valider le code d’entraînement et le FNO avec des champs lisses et des contraintes PDE cohérentes avec la loss PINN.

Génération puis entraînement (grille **puissances de 2** pour compatibilité FFT / GPU) :

```bash
# Depuis la racine du monorepo (venv ou environnement avec torch, h5py, scipy, pyyaml)
PYTHONPATH=. python -m training.src.data.generate_synthetic_dataset \
  --output training/data/synthetic_cfd_local.h5 \
  --meta-yaml training/configs/local_synthetic.yaml \
  --n-train 24 --n-val 6 --seed 42

PYTHONPATH=. python -m training.src.model.train --config training/configs/local_synthetic.yaml
```

Les sorties (`training/data/*.h5`, `training/checkpoints/`, `training/runs/`) sont listées dans `.gitignore` et ne doivent pas être versionnées.

**Note technique** : la couche spectrale du FNO n’est pas compatible avec l’AMP fp16 sur CUDA ; `use_amp` est **désactivé** par défaut dans `train.py` (voir `training.use_amp` dans le YAML si un jour le modèle le supporte).

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

**Intégration backend** (`back/models/`) : générer le graphe mock, puis copier les stats de normalisation **réelles** de l’entraînement (le FNO entier ne s’exporte pas en ONNX à cause de `fft_rfftn`) :

```bash
PYTHONPATH=. python -m training.src.model.mock_onnx --output back/models/klima.onnx --norm-output /tmp/mock.json
cp training/checkpoints/norm_params.json back/models/norm_params.json
```

Voir `back/models/README.md` pour le détail et `training/src/model/export_onnx.py` (`dynamo=False` ; échec attendu sur le vrai FNO tant que l’export FFT n’existe pas).

## Inférence PyTorch (checkpoint entraîné)

Sidecar **`training/infer_server/`** : charge `best_model.pt` + `norm_params.json`. Démarrage avec **`./scripts/run.sh dev-infer`** (fixe `KLIMA_FNO_URL` ; service `klima-infer`, port hôte **8001**). Voir [infer_server/README.md](infer_server/README.md).
