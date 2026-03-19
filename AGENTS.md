# Klima — Agent Instructions

## Cursor Cloud specific instructions

### Architecture

Klima is a Docker-based monorepo with three runtime services:

| Service | Container | Port | Tech |
|---------|-----------|------|------|
| Backend API | `klima-back` | 3000 | Rust / Axum / ONNX Runtime |
| Frontend | `klima-front` | 9000 | Vue 3 / Quasar / CesiumJS |
| Database | `klima-db` | 5432 | PostgreSQL 16 |

An optional `klima-training` service (Python/PyTorch) exists for ML model training but requires an NVIDIA GPU and is not needed for dev.

### Starting the dev environment

```bash
sudo dockerd &>/tmp/dockerd.log &
sleep 3
sudo chmod 666 /var/run/docker.sock
./scripts/run-dev.sh up
```

Docker must be started manually in the Cloud VM since there's no systemd. After `run-dev.sh up`, the first Rust compilation takes ~10 minutes (cached afterward via Docker volumes `cargo-cache` and `target-cache`).

### Key caveats

- **Backend Dockerfile**: `back/docker/Dockerfile.dev` uses `rust:trixie` (Debian 13, glibc 2.40) because the `ort` crate's pre-built ONNX Runtime binary requires glibc >= 2.38. The original `rust:bookworm` (glibc 2.36) causes linker errors.
- **ONNX model not required**: The backend gracefully falls back to mock data when no `.onnx` model is loaded. The simulate endpoint returns mock results.
- **Cesium Ion token**: Optional. Without `CESIUM_ION_TOKEN`, the 3D map shows a starfield instead of terrain/buildings. Pass it via: `CESIUM_ION_TOKEN=xxx ./scripts/run-dev.sh up`
- **Frontend proxy**: The frontend dev server proxies `/api/*` to `http://klima-back:3000` via Docker networking. For local (non-Docker) testing, use `VITE_API_BASE_URL=http://localhost:3000`.

### Lint / Test / Build

| Check | Command |
|-------|---------|
| Backend check | `docker exec klima-back cargo check` |
| Backend test | `docker exec klima-back cargo test` |
| Frontend lint | `docker exec klima-front npx eslint ./src` |
| Frontend build | `docker exec klima-front npm run build` |

### Useful API endpoints

- `GET /api/health` — returns `{"status":"ok","version":"0.1.0"}`
- `GET /api/projects` — list projects
- `POST /api/projects` — create project (`{"name":"...","description":"..."}`)
- `POST /api/projects/{id}/scenarios` — create scenario (`{"name":"...","geometry":{...}}`)
- `POST /api/scenarios/{id}/simulate` — run simulation (returns mock data without ONNX model)
