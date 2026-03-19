# Architecture du Projet Klima

## Vue d'ensemble

```
┌────────────────────────────────────────────────────────┐
│                    Monorepo klima/                      │
│                                                        │
│  ┌──────────────────┐       ┌───────────────────────┐  │
│  │    back/ (Rust)   │       │  front/ (Vue/Quasar)  │  │
│  │                   │       │                       │  │
│  │  Axum API :3000   │◄─────►│  Quasar Dev :9000     │  │
│  │  ONNX Runtime     │ JSON  │  CesiumJS 3D          │  │
│  │  SQLite           │       │                       │  │
│  └──────────────────┘       └───────────────────────┘  │
│           │                          │                  │
│    back/docker/               front/docker/             │
│    Dockerfile.dev             Dockerfile.dev            │
│    docker-compose.dev.yml     docker-compose.dev.yml    │
│                                                        │
│  scripts/run-dev.sh → lance les deux containers         │
└────────────────────────────────────────────────────────┘
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
│   │       └── mod.rs          # SQLite (rusqlite)
│   └── docker/
│       ├── Dockerfile.dev
│       └── docker-compose.dev.yml
│
├── front/                      # Frontend Vue.js
│   ├── package.json
│   ├── quasar.config.ts
│   ├── tsconfig.json
│   ├── index.html
│   ├── src/
│   │   ├── main.ts
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
│   ├── specification.md
│   ├── architecture.md
│   └── setup.md
│
├── scripts/
│   └── run-dev.sh              # Lance l'env de dev Docker
│
├── .gitignore
└── README.md
```

## Flux de données

1. L'utilisateur interagit avec la carte CesiumJS (modifie la géométrie, ajuste les paramètres météo).
2. Le frontend envoie un `POST /api/simulate` avec la géométrie et les paramètres.
3. Le backend Rust transforme les données en tenseur, les passe au modèle ONNX.
4. Le modèle retourne les matrices de température et flux d'air.
5. Le backend renvoie les résultats au frontend en JSON compressé.
6. Le frontend superpose les résultats sur CesiumJS (colorisation thermique, particules de vent).

## Communication inter-services (Docker)

- Les deux containers partagent le réseau Docker `klima-net`.
- Le frontend proxy les appels `/api/*` vers `http://klima-back:3000`.
- Chaque container monte le code source du host via un volume partagé.
- Les dépendances (node_modules, cargo registry, target) sont dans des volumes Docker nommés pour la performance.
