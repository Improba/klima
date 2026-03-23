# Klima — Training Pipeline

Training pipeline for the urban microclimate 3D simulator using a Local Fourier Neural Operator (FNO).

**PyTorch et les dépendances d’entraînement sont dans l’image Docker** (`training/docker/Dockerfile`). Les agents et les CI peuvent valider torch + les modules Python **sans rien installer sur l’hôte** :

```bash
cd training/docker && docker compose run --rm klima-training-verify
```

## Quick Start (Docker, recommandé)

Requires [Docker](https://docs.docker.com/get-docker/) and, pour l’entraînement GPU, [NVIDIA Container Toolkit](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/install-guide.html).

- Compose **project** `klima-training` : `training/docker/docker-compose.yml`.
- Le dépôt est monté à `/app`, `PYTHONPATH=/app` ; le service `klima-training` a `working_dir=/app/training` (chemins relatifs `data/…`, `checkpoints/` dans les YAML).

Vérifier PyTorch + imports / forward minimal de la loss PINN :

```bash
cd training/docker && docker compose run --rm klima-training-verify
```

Entraînement (GPU, `gpus: all` sur le service `klima-training`) :

```bash
cd training/docker && docker compose up --build
```

`default.yaml` attend un jeu CFD réel à `training/data/cfd_dataset.h5`. Pour des essais **sans OpenFOAM**, voir les données synthétiques ci‑dessous et `local_synthetic*.yaml`.

## Optionnel : Python sur l’hôte (sans Docker)

Si vous maintenez un venv local avec **torch** installé manuellement :

```bash
pip install -r training/requirements.txt
PYTHONPATH=. python -m training.src.model.train --config training/configs/default.yaml
```

Préférez Docker pour reproduire la même stack que l’équipe.

## Données synthétiques (physique explicite, hors CFD)

Le module `training/src/data/synthetic_physics.py` construit des paires entrée/sortie compatibles avec le chargeur HDF5 :

- **Température** : Laplace ∇²*T* = 0 dans l’air, températures imposées dans les solides et sur le pourtour du domaine (conductif stationnaire simplifié).
- **Vent** : champ **à divergence nulle** *v* = ∇×**B** (**B** = bruit gaussien lissé) + vent moyen horizontal selon la direction méto.

Ce n’est **pas** un substitut à un maillage CFD urbain, mais une base reproductible pour valider le code d’entraînement et le FNO avec des champs lisses et des contraintes PDE cohérentes avec la loss PINN.

Génération puis entraînement (grille **puissances de 2** pour compatibilité FFT / GPU) — **via Docker** :

```bash
# Génération : service sans gpus (CPU suffit)
cd training/docker && docker compose run --rm -w /app -e PYTHONPATH=/app klima-training-verify \
  python -m training.src.data.generate_synthetic_dataset \
  --output training/data/synthetic_cfd_local.h5 \
  --meta-yaml training/configs/local_synthetic.yaml \
  --n-train 24 --n-val 6 --seed 42

# Entraînement : `working_dir` = racine du monorepo (`/app`) car les YAML utilisent `training/data/...`
# GPU : `klima-training` ; sans GPU : `klima-training-verify` (CPU, plus lent).
cd training/docker && docker compose run --rm -w /app -e PYTHONPATH=/app klima-training \
  python -m training.src.model.train --config training/configs/local_synthetic.yaml
```

Équivalent **hôte** (si venv torch déjà installé) : mêmes commandes `PYTHONPATH=. python -m …` depuis la racine du monorepo.

Les sorties (`training/data/*.h5`, `training/checkpoints/`, `training/runs/`) sont listées dans `.gitignore` et ne doivent pas être versionnées.

**Note technique** : la couche spectrale du FNO n’est pas compatible avec l’AMP fp16 sur CUDA ; `use_amp` est **désactivé** par défaut dans `train.py` (voir `training.use_amp` dans le YAML si un jour le modèle le supporte).

### Détails Docker

- Compose **project name** `klima-training` (séparé de la stack dev `klima`).
- **`klima-training-verify`** : même image, **sans** `gpus:` — smoke test torch + PINN sur CPU ou GPU selon l’hôte (import et petit forward).
- **`klima-training`** : `gpus: all` pour l’entraînement. Si erreur *could not select device driver "nvidia"* : [NVIDIA Container Toolkit](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/install-guide.html), puis `sudo nvidia-ctk runtime configure --runtime=docker` et redémarrer Docker.

### Le GPU est-il « dans l’image » ?

**Non au sens matériel** : l’image contient les **bibliothèques** CUDA/cuDNN et PyTorch ; le **device** est celui de l’hôte, exposé avec `gpus: all` sur le service d’entraînement.

Vérifications après `cd training/docker` :

```bash
docker compose run --rm klima-training-verify   # torch + code training (sans GPU obligatoire)
chmod +x check-gpu.sh && ./check-gpu.sh         # + nvidia-smi + torch.cuda dans le conteneur GPU
```

**Génération synthétique + train court** (one-shot) :

```bash
cd training/docker && docker compose run --rm -w /app -e PYTHONPATH=/app klima-training-verify \
  python -m training.src.data.generate_synthetic_dataset \
  --output training/data/synthetic_cfd_local.h5 \
  --meta-yaml training/configs/local_synthetic_quick.yaml \
  --n-train 16 --n-val 4 --seed 42
cd training/docker && docker compose run --rm -w /app -e PYTHONPATH=/app klima-training \
  python -m training.src.model.train --config training/configs/local_synthetic_quick.yaml
```

## Mock ONNX (for backend dev)

Dans l’image d’entraînement :

```bash
cd training/docker && docker compose run --rm -w /app -e PYTHONPATH=/app klima-training-verify \
  python -m training.src.model.mock_onnx --output model.onnx --norm-output norm_params.json
```

Ou sur l’hôte avec venv torch : `PYTHONPATH=. python -m training.src.model.mock_onnx …`

**Intégration backend** (`back/models/`) : générer le graphe mock, puis copier les stats de normalisation **réelles** de l’entraînement (le FNO entier ne s’exporte pas en ONNX à cause de `fft_rfftn`) :

```bash
PYTHONPATH=. python -m training.src.model.mock_onnx --output back/models/klima.onnx --norm-output /tmp/mock.json
cp training/checkpoints/norm_params.json back/models/norm_params.json
```

Voir `back/models/README.md` pour le détail et `training/src/model/export_onnx.py` (`dynamo=False` ; échec attendu sur le vrai FNO tant que l’export FFT n’existe pas).

## Inférence PyTorch (checkpoint entraîné)

Sidecar **`training/infer_server/`** : charge `best_model.pt` + `norm_params.json`. Démarrage avec **`./scripts/run.sh dev-infer`** (fixe `KLIMA_FNO_URL` ; service `klima-infer`, port hôte **8001**). Voir [infer_server/README.md](infer_server/README.md).
