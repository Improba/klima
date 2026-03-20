# Guide d'Installation — Klima

## Prérequis

- [Docker](https://docs.docker.com/get-docker/) >= 24.0
- [Docker Compose](https://docs.docker.com/compose/install/) >= 2.20

Aucune installation locale de Rust, Node.js ou autre n'est nécessaire pour **l’application** (backend + frontend + PostgreSQL) : tout tourne dans Docker via `./scripts/run.sh`.

## Démarrage rapide

```bash
# Cloner le dépôt
git clone git@github.com:Improba/klima.git
cd klima

# Lancer l'environnement de développement
./scripts/run.sh
```

Les services sont ensuite accessibles :

| Service | URL |
|---------|-----|
| Backend (API Rust) | http://localhost:3000 |
| Frontend (Quasar) | http://localhost:9000 |
| PostgreSQL | localhost:5432 (user: klima, pass: klima, db: klima) |

## Commandes utiles

### Entrer dans un container

```bash
# Backend (Rust)
docker exec -it klima-back bash

# Frontend (Node/Vue)
docker exec -it klima-front bash
```

### Depuis le container backend

```bash
cargo build              # Compiler
cargo test               # Lancer les tests
cargo add <crate>        # Ajouter une dépendance
```

### Depuis le container frontend

```bash
npm install              # Installer les dépendances
npx quasar dev           # Serveur de dev (déjà lancé)
npm run build            # Build production
npm install <package>    # Ajouter un package
```

### Gestion des containers

```bash
./scripts/run.sh              # Démarrer (DB + back + front, sans sidecar FNO)
./scripts/run.sh dev-infer    # Idem + sidecar PyTorch FNO (port 8001)
./scripts/run.sh down         # Arrêter
./scripts/run.sh down -v      # Arrêter et supprimer les volumes nommés
./scripts/run.sh logs         # Voir les logs
./scripts/run.sh restart      # Redémarrer (sans infer)
./scripts/run.sh restart-infer # Redémarrer avec le sidecar FNO
```

## Variables d'environnement

| Variable | Description | Où la définir |
|----------|-------------|---------------|
| `CESIUM_ION_TOKEN` | Token Cesium Ion (optionnel) pour les bâtiments 3D Cesium OSM | Fichier **`.env` à la racine** du repo (chargé par `./scripts/run.sh`) ou `export` dans le shell |
| `KLIMA_FNO_URL` | URL du sidecar PyTorch FNO | Vide par défaut (`./scripts/run.sh`) pour un dev fluide ; défini automatiquement avec `./scripts/run.sh dev-infer` |
| `RUST_LOG`, `KLIMA_MODEL_PATH`, `KLIMA_NORM_PATH`, `KLIMA_CACHE_SIZE` | Backend | Surcharges via `back/docker/.env` (voir `back/docker/.env.example`) ou shell avant `docker compose` |

Un **`.env.example`** à la racine et **`back/docker/.env.example`** documentent les clés utiles.

Si tu configures **`KLIMA_FNO_URL` dans `back/docker/.env`**, ajoute **`COMPOSE_PROFILES=infer`** (ou utilise `./scripts/run.sh dev-infer`) ; sinon le backend tentera d’appeler un sidecar non démarré.

## Configuration Cesium Ion

L’imagery de base vient d’OpenStreetMap (pas besoin de token). Pour activer les **bâtiments 3D** (`createOsmBuildingsAsync`), un token Cesium Ion (gratuit) est nécessaire :

1. Créer un compte sur https://ion.cesium.com
2. Générer un token d'accès
3. Le fournir au lancement :

```bash
# Soit variable d’environnement :
CESIUM_ION_TOKEN=your_token_here ./scripts/run.sh
# Soit fichier `.env` à la racine du repo (recommandé) : CESIUM_ION_TOKEN=...
```

## Entraînement du modèle (optionnel)

Hors du script `run.sh` : stack séparée **Docker Compose**, projet `klima-training`, conteneur `klima-training`. Nécessite un **GPU NVIDIA** et le [NVIDIA Container Toolkit](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/install-guide.html).

```bash
cd training/docker && docker compose up --build
```

Le dépôt entier est monté dans le conteneur (`PYTHONPATH` = racine du monorepo, répertoire de travail = `training/`). Détail des prérequis données, export ONNX et dépannage `runtime: nvidia` : [training/README.md](../training/README.md).

Pour un **jeu de données local sans CFD** (Laplace + vent divergence nulle, voir doc du module), suivre la section *Données synthétiques* du même README ; les fichiers `.h5`, checkpoints et runs restent ignorés par Git.

**Inférence FNO entraînée (PyTorch)** : lancer **`./scripts/run.sh dev-infer`** (démarre `klima-infer` + fixe `KLIMA_FNO_URL`). Sidecar sur **http://localhost:8001** ; checkpoints sous `training/checkpoints/` (`best_model.pt` + `norm_params.json`, voir `training/infer_server/README.md`). Le **`./scripts/run.sh`** classique ne démarre pas le sidecar et laisse `KLIMA_FNO_URL` vide → pas d’appel HTTP inutile, ONNX puis mock.
