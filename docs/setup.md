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
./scripts/run.sh              # Démarrer (mode dev)
./scripts/run.sh down         # Arrêter
./scripts/run.sh down -v      # Arrêter et supprimer les volumes nommés
./scripts/run.sh logs         # Voir les logs
./scripts/run.sh restart      # Redémarrer
```

## Variables d'environnement

| Variable | Description | Où la définir |
|----------|-------------|---------------|
| `CESIUM_ION_TOKEN` | Token Cesium Ion (optionnel) pour les bâtiments 3D Cesium OSM | `.env` à la racine ou export shell |
| `RUST_LOG` | Niveau de log du backend | docker-compose (par défaut: `debug`) |

## Configuration Cesium Ion

L’imagery de base vient d’OpenStreetMap (pas besoin de token). Pour activer les **bâtiments 3D** (`createOsmBuildingsAsync`), un token Cesium Ion (gratuit) est nécessaire :

1. Créer un compte sur https://ion.cesium.com
2. Générer un token d'accès
3. Le fournir au lancement :

```bash
CESIUM_ION_TOKEN=your_token_here ./scripts/run.sh
```

## Entraînement du modèle (optionnel)

Hors du script `run.sh` : stack séparée **Docker Compose**, projet `klima-training`, conteneur `klima-training`. Nécessite un **GPU NVIDIA** et le [NVIDIA Container Toolkit](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/install-guide.html).

```bash
cd training/docker && docker compose up --build
```

Le dépôt entier est monté dans le conteneur (`PYTHONPATH` = racine du monorepo, répertoire de travail = `training/`). Détail des prérequis données, export ONNX et dépannage `runtime: nvidia` : [training/README.md](../training/README.md).
