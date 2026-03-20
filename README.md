<div align="center">

# Klima

**Simulateur IA *surrogate* de microclimat urbain en 3D**

Visualisez et explorez l’impact thermique des aménagements (îlots de chaleur, flux d’air) via un modèle de substitution entraîné — une alternative rapide aux chaînes CFD classiques.

[![Rust](https://img.shields.io/badge/API-Rust-CE422B?logo=rust&logoColor=white)](back/)
[![Vue](https://img.shields.io/badge/UI-Vue%203-42b883?logo=vuedotjs&logoColor=white)](front/)
[![Docker](https://img.shields.io/badge/dev-Docker-2496ED?logo=docker&logoColor=white)](./scripts/run.sh)

</div>

---

> **Statut : phase de test et de recherche**  
> Klima est un **laboratoire logiciel** : l’API, l’interface et le pipeline modèle évoluent encore. Ce dépôt **ne constitue pas un produit fini** prêt pour la production. Les résultats de simulation peuvent être **indicatifs ou simulés** (données de secours sans modèle ONNX, comportements sujets à changement). Utilisez-le pour **expérimenter et contribuer**, pas comme référence métier figée.

---

## Stack technique

Le monorepo couvre l’API et l’inférence, l’interface 3D et le pipeline d’entraînement du modèle.

| Couche | Technologie |
|--------|-------------|
| Backend API & inférence | **Rust** — Axum, ONNX Runtime (`ort`), PostgreSQL |
| Modèle IA | Local-FNO (Fourier Neural Operator) + PINN, export ONNX |
| Frontend & 3D | **Vue.js 3** — Quasar, CesiumJS |
| Entraînement | Python, PyTorch, NVIDIA Modulus / neuraloperator |
| Dev | **Docker** + Docker Compose |

---

## Démarrage rapide

```bash
# Prérequis : Docker >= 24.0, Docker Compose >= 2.20

git clone git@github.com:Improba/klima.git
cd klima
./scripts/run.sh
```

| Service | URL |
|--------|-----|
| API (backend) | http://localhost:3000 |
| Interface web | http://localhost:9000 |

**Optionnel** : copier `.env.example` vers **`.env`** à la racine et y mettre `CESIUM_ION_TOKEN` (le script `run.sh` charge ce fichier). Ou **`./scripts/run.sh dev-infer`** pour le sidecar PyTorch FNO (port 8001) en plus de l’API. Sans token Cesium, la carte reste utilisable (OSM + globe sombre).

---

## Structure du monorepo

```
klima/
├── back/           API, inférence ONNX, PostgreSQL
├── front/          Interface web et scène 3D
├── training/       Entraînement du modèle (optionnel, voir training/README.md)
├── docs/           Documentation
├── scripts/        Orchestration Docker
└── README.md
```

---

## Commandes courantes

```bash
# Shell dans les conteneurs
docker exec -it klima-back  bash   # cargo, etc.
docker exec -it klima-front bash   # npm, quasar, etc.

# Cycle de vie
./scripts/run.sh              # Démarrer (DB + API + UI, sans sidecar FNO)
./scripts/run.sh dev-infer    # + sidecar PyTorch FNO (localhost:8001)
./scripts/run.sh down         # Arrêter
./scripts/run.sh down -v      # Arrêter + volumes
./scripts/run.sh logs         # Logs
./scripts/run.sh restart      # Redémarrer
./scripts/run.sh restart-infer # Redémarrer avec FNO

# Entraînement ML (optionnel, GPU — stack Compose séparée, voir training/README.md)
# cd training/docker && docker compose up --build
```

---

## Documentation

- [Spécification](docs/specification.md)
- [Architecture](docs/architecture.md)
- [Installation détaillée](docs/setup.md)

---

<div align="center">

*Klima — expérimentation microclimat urbain · Improba*

</div>
