# Guide d'Installation — Klima

## Prérequis

- [Docker](https://docs.docker.com/get-docker/) >= 24.0
- [Docker Compose](https://docs.docker.com/compose/install/) >= 2.20

Aucune installation locale de Rust, Node.js ou autre n'est nécessaire. Tout tourne dans Docker.

## Démarrage rapide

```bash
# Cloner le dépôt
git clone git@github.com:Improba/klima.git
cd klima

# Lancer l'environnement de développement
./scripts/run-dev.sh up
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
./scripts/run-dev.sh up       # Démarrer
./scripts/run-dev.sh down     # Arrêter
./scripts/run-dev.sh logs     # Voir les logs
./scripts/run-dev.sh restart  # Redémarrer
```

## Variables d'environnement

| Variable | Description | Où la définir |
|----------|-------------|---------------|
| `CESIUM_ION_TOKEN` | Token Cesium Ion pour les tuiles 3D | `.env` à la racine ou export shell |
| `RUST_LOG` | Niveau de log du backend | docker-compose (par défaut: `debug`) |

## Configuration Cesium Ion

Pour avoir accès aux bâtiments 3D et au terrain, il faut un token Cesium Ion (gratuit) :

1. Créer un compte sur https://ion.cesium.com
2. Générer un token d'accès
3. Le fournir au lancement :

```bash
CESIUM_ION_TOKEN=your_token_here ./scripts/run-dev.sh up
```
