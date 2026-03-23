# Klima — Agent Instructions

**Ne pas dupliquer la doc ici.** S’appuyer sur les fichiers du dépôt :

| Sujet | Fichier |
|--------|---------|
| Vue d’ensemble, démarrage | [README.md](README.md) |
| Installation, Docker, commandes dev | [docs/setup.md](docs/setup.md) |
| Entraînement PyTorch / Compose `klima-training` | [training/README.md](training/README.md) |
| Sidecar FNO (`dev-infer`) | [training/infer_server/README.md](training/infer_server/README.md) |
| ONNX backend | [back/models/README.md](back/models/README.md) |
| Spec / architecture (réf.) | [docs/specification.md](docs/specification.md), [docs/architecture.md](docs/architecture.md) |

## Cursor Cloud

Pas de systemd : démarrer le daemon Docker, puis `./scripts/run.sh` — détail dans [docs/setup.md](docs/setup.md).
