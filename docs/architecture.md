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
│  │  ONNX Runtime (FNO)   │ JSON  │  CesiumJS 3D                  │  │
│  │  GeoJSON→Tensor       │       │  GeoJSON draw tools            │  │
│  │  PostgreSQL (sqlx)    │       │                                │  │
│  └──────────┬────────────┘       └────────────────────────────────┘  │
│             │                                                        │
│  ┌──────────▼────────────┐                                          │
│  │   klima-db (Postgres)  │                                          │
│  │   PostgreSQL 16 :5432  │                                          │
│  └────────────────────────┘                                          │
│                                                                     │
│  training/  →  Python (PyTorch, NVIDIA Modulus, neuraloperator)      │
│                Local-FNO + PINN  →  export .onnx                    │
│                                                                     │
│  scripts/run.sh → lance les containers Docker                        │
└─────────────────────────────────────────────────────────────────────┘
```

## Structure des dossiers

```
klima/
├── back/                       # Backend Rust
│   ├── Cargo.toml
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
│       └── docker-compose.dev.yml  # back + klima-db (PostgreSQL 16)
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
├── docs/                       # Documentation
│   ├── specification.md        # Spec (FNO, PINN, GeoJSON-to-Tensor)
│   ├── architecture.md         # Ce fichier
│   ├── setup.md
│   └── plans/
│       └── implementation-plan.md
│
├── scripts/
│   └── run.sh                  # Lance l'env de dev Docker
│
├── .gitignore
└── README.md
```

## Stack technique

| Couche | Technologie | Rôle |
|--------|-------------|------|
| Modèle IA | **Local-FNO** (Fourier Neural Operator) + contraintes PINN | Prédiction ΔT et v en temps réel, zero-shot super-résolution |
| Entraînement | Python, PyTorch, NVIDIA Modulus / neuraloperator | Entraînement sur 24-50 sims CFD, export ONNX |
| Backend | **Rust** — Axum, `ort` (ONNX Runtime), `sqlx` | API REST, inférence ONNX, pipeline GeoJSON→Tensor |
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
      │                                │                                │ Inférence FNO (ONNX)
      │                                │                                │ ΔT + v en < 200 ms
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

- 3 containers : `klima-back` (Rust), `klima-front` (Node), `klima-db` (PostgreSQL)
- Réseau Docker `klima-net` partagé
- Le backend se connecte à PostgreSQL via `DATABASE_URL=postgres://klima:klima@klima-db:5432/klima`
- Le frontend proxy les appels `/api/*` vers `http://klima-back:3000`
- Chaque container applicatif monte le code source du host via un volume partagé
- PostgreSQL persiste ses données dans un volume Docker nommé `pgdata`
