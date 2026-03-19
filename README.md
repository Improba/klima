# Klima

Simulateur IA *Surrogate* de microclimat urbain 3D.

Application web permettant de simuler en temps réel l'impact thermique des aménagements urbains (îlots de chaleur, flux d'air) grâce à un modèle d'IA de substitution, en remplacement des simulations CFD traditionnelles.

## Stack technique

| Couche | Technologie |
|--------|-------------|
| Backend API & Inférence | **Rust** — Axum, ONNX Runtime (`ort`), SQLite |
| Frontend Visualisation 3D | **Vue.js 3** — Quasar, CesiumJS |
| Modèle IA | **ONNX** (entraîné via PyTorch) |
| Infrastructure dev | **Docker** + Docker Compose |

## Démarrage rapide

```bash
# Prérequis : Docker >= 24.0, Docker Compose >= 2.20

git clone git@github.com:Improba/klima.git
cd klima
./scripts/run-dev.sh up
```

- Backend : http://localhost:3000
- Frontend : http://localhost:9000

## Structure du monorepo

```
klima/
├── back/           Rust / Axum — API + inférence ONNX + SQLite
├── front/          Vue.js / Quasar / CesiumJS — interface 3D
├── docs/           Documentation projet
├── scripts/        Scripts d'orchestration
└── README.md
```

## Commandes courantes

```bash
# Entrer dans les containers
docker exec -it klima-back  bash   # pour cargo build, cargo add, etc.
docker exec -it klima-front bash   # pour npm install, quasar dev, etc.

# Gestion
./scripts/run-dev.sh up            # Démarrer
./scripts/run-dev.sh down          # Arrêter
./scripts/run-dev.sh logs          # Logs
./scripts/run-dev.sh restart       # Redémarrer
```

## Documentation

- [Spécification complète](docs/specification.md)
- [Architecture technique](docs/architecture.md)
- [Guide d'installation](docs/setup.md)
