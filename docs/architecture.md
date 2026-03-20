# Architecture du Projet Klima

## Vue d'ensemble

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Monorepo klima/                               │
│                                                                     │
│  ┌──────────────────────┐       ┌────────────────────────────────┐  │
│  │    back/ (Rust)       │       │     front/ (Vue/Quasar)        │  │
│  │                       │       │                                │  │
│  │  Axum API :3000       │◄─────►│  Quasar Dev :9000              │  │
│  │  FNO sidecar / ONNX   │ JSON  │  CesiumJS 3D                  │  │
│  │  GeoJSON→Tensor       │       │  GeoJSON draw tools            │  │
│  │  PostgreSQL (sqlx)    │       │                                │  │
│  └──────────┬────────────┘       └────────────────────────────────┘  │
│             │                                                        │
│  ┌──────────▼────────────┐                                          │
│  │   klima-db (Postgres)  │                                          │
│  │   PostgreSQL 16 :5432  │                                          │
│  └────────────────────────┘                                          │
│                                                                     │
│  training/  →  Python (PyTorch) : entraînement + infer_server (FNO .pt) │
│                ONNX optionnel (mock si FFT non exportable)            │
│                                                                     │
│  scripts/run.sh → back + front + DB (+ optionnel `klima-infer` FNO)   │
│  training/docker → entraînement GPU optionnel (projet `klima-training`) │
└─────────────────────────────────────────────────────────────────────┘
```

## Structure des dossiers

```
klima/
├── back/                       # Backend Rust
│   ├── Cargo.toml
│   ├── migrations/             # Schéma PostgreSQL (sqlx migrate)
│   ├── src/
│   │   ├── main.rs             # Point d'entrée, serveur Axum
│   │   ├── routes/
│   │   │   ├── mod.rs          # Routeur API
│   │   │   ├── health.rs       # GET /api/health
│   │   │   └── simulate.rs     # POST /api/simulate
│   │   └── db/
│   │       └── mod.rs          # PostgreSQL (sqlx + PgPool)
│   └── docker/
│       ├── Dockerfile.dev
│       └── docker-compose.dev.yml  # back + db (+ optionnel klima-infer)
│
├── front/                      # Frontend Vue.js
│   ├── package.json
│   ├── quasar.config.ts
│   ├── tsconfig.json
│   ├── index.html
│   ├── src/
│   │   ├── App.vue
│   │   ├── boot/
│   │   │   └── cesium.ts       # Config Cesium Ion token
│   │   ├── components/
│   │   │   └── CesiumViewer.vue # Viewer 3D (canvas)
│   │   ├── layouts/
│   │   │   └── MainLayout.vue  # Layout Quasar + panneaux
│   │   ├── pages/
│   │   │   ├── IndexPage.vue
│   │   │   └── ErrorNotFound.vue
│   │   ├── router/
│   │   │   ├── index.ts
│   │   │   └── routes.ts
│   │   └── css/
│   │       ├── app.scss
│   │       └── quasar.variables.scss
│   └── docker/
│       ├── Dockerfile.dev
│       └── docker-compose.dev.yml
│
├── training/                   # Pipeline ML (optionnel, GPU)
│   ├── configs/
│   ├── docker/
│   │   ├── Dockerfile          # PyTorch CUDA
│   │   └── docker-compose.yml  # Projet Compose `klima-training`
│   ├── infer_server/           # Sidecar FastAPI FNO (wire binaire KLM1)
│   ├── src/
│   └── README.md
│
├── docs/                       # Documentation
│   ├── specification.md        # Spec (FNO, PINN, GeoJSON-to-Tensor)
│   ├── architecture.md         # Ce fichier
│   ├── setup.md
│   └── plans/
│       └── implementation-plan.md
│
├── scripts/
│   └── run.sh                  # Dev Docker (back + front + DB) ; dev-infer + sidecar FNO
│
├── .gitignore
└── README.md
```

## Stack technique

| Couche | Technologie | Rôle |
|--------|-------------|------|
| Modèle IA | **Local-FNO** (Fourier Neural Operator) + contraintes PINN | Prédiction ΔT et v en temps réel, zero-shot super-résolution |
| Entraînement | Python, PyTorch | Entraînement FNO ; inférence via **infer_server** ou export ONNX si possible |
| Backend | **Rust** — Axum, `ort` (ONNX Runtime), `reqwest`, `sqlx` | API REST ; chaîne **FNO (HTTP) → ONNX → mock**, pipeline GeoJSON→Tensor |
| Base de données | **PostgreSQL 16** | Projets, scénarios, résultats (JSONB + BYTEA) |
| Frontend | **Vue.js 3** — Quasar, CesiumJS | Interface 3D, outils de dessin GeoJSON, visualisation |
| Infrastructure | Docker, Docker Compose | Conteneurisation de tous les services |

## Flux de données

### Flux principal (simulation interactive)

```
  Utilisateur                    Frontend (CesiumJS)              Backend (Rust/Axum)
      │                                │                                │
      │ dessine un polygone            │                                │
      │ (toit végétalisé)              │                                │
      ├───────────────────────────────►│                                │
      │                                │  GeoJSON + params météo        │
      │                                ├───────────────────────────────►│
      │                                │                                │ GeoJSON → Tensor
      │                                │                                │ (rasterisation sur grille voxel)
      │                                │                                │
│                                │                                │ FNO sidecar PyTorch, sinon ONNX, sinon mock
│                                │                                │ ΔT + v (latence selon modèle)
      │                                │                                │
      │                                │  surface_temps + wind_field    │
      │                                │◄───────────────────────────────┤
      │  heatmap 3D + particules vent  │                                │
      │◄───────────────────────────────┤                                │
```

### Flux zoom (super-résolution)

```
  Utilisateur zoome ×2            Frontend                          Backend
      │                                │                                │
      │                                │  bbox + résolution cible       │
      │                                ├───────────────────────────────►│
      │                                │                                │ FNO évalue sur grille 2×
      │                                │                                │ (dynamic axes ONNX)
      │                                │                                │ Zero-shot, pas de ré-entraînement
      │                                │  résultat haute-résolution     │
      │                                │◄───────────────────────────────┤
      │  rendu détaillé                │                                │
      │◄───────────────────────────────┤                                │
```

## Communication inter-services (Docker)

### Application (développement)

- Containers : `klima-back` (Rust), `klima-front` (Node), `klima-db` (PostgreSQL). **`klima-infer`** (FastAPI + PyTorch) est optionnel : profil Compose `infer`, démarré par **`./scripts/run.sh dev-infer`** (pas par `./scripts/run.sh` seul).
- Réseau Docker `klima-net` partagé
- Le backend se connecte à PostgreSQL via `DATABASE_URL=postgres://klima:klima@klima-db:5432/klima`
- Inférence simulation : si `KLIMA_FNO_URL` est défini (ex. après `dev-infer`), `/api/simulate` appelle d’abord le sidecar ; sinon **ONNX** puis **mock**. Par défaut le dev standard ne définit pas `KLIMA_FNO_URL`.
- Le frontend proxy les appels `/api/*` vers `http://klima-back:3000`
- Chaque container applicatif monte le code source du host via un volume partagé
- PostgreSQL persiste ses données dans un volume Docker nommé `pgdata`

### Entraînement (optionnel)

- Fichier `training/docker/docker-compose.yml`, projet Compose **`klima-training`** (distinct de `klima` pour éviter les collisions avec la stack dev).
- Conteneur **`klima-training`** : montage de la **racine du monorepo** dans `/app`, `PYTHONPATH=/app`, `working_dir=/app/training` pour les chemins du config YAML.
- Pas de réseau partagé avec `klima-net` par défaut ; l’API consomme un modèle ONNX via variables d’environnement (`KLIMA_MODEL_PATH`, `KLIMA_NORM_PATH`) côté backend si tu les configures.
