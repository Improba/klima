# Klima — Agent Instructions

## Cursor Cloud specific instructions

### Architecture

Klima is a Docker-based monorepo with three core runtime services (plus optional FNO sidecar):

| Service | Container | Port | Tech |
|---------|-----------|------|------|
| Backend API | `klima-back` | 3000 | Rust / Axum / ONNX Runtime |
| Frontend | `klima-front` | 9000 | Vue 3 / Quasar / CesiumJS |
| Database | `klima-db` | 5432 | PostgreSQL 16 |
| FNO infer (optional) | `klima-infer` | 8001→8000 | FastAPI / PyTorch — `./scripts/run.sh dev-infer` |

An optional **training** stack (Python/PyTorch, CUDA) is **not** started by `run.sh`. Use `training/docker/docker-compose.yml` (Compose project `klima-training`, container `klima-training`). It mounts the **monorepo root** at `/app`, sets `PYTHONPATH=/app`, and `working_dir=/app/training`. Requires an NVIDIA GPU and the [NVIDIA Container Toolkit](https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/install-guide.html). See `training/README.md` for local vs Docker commands and `runtime: nvidia` troubleshooting.

### Starting the dev environment

```bash
sudo dockerd &>/tmp/dockerd.log &
sleep 3
sudo chmod 666 /var/run/docker.sock
./scripts/run.sh
```

Docker must be started manually in the Cloud VM since there's no systemd. After `./scripts/run.sh`, the first Rust compilation takes ~10 minutes (cached afterward via Docker volumes `cargo-cache` and `target-cache`).

### Key caveats

- **Backend Dockerfile**: `back/docker/Dockerfile.dev` uses `rust:trixie` (Debian 13, glibc 2.40) because the `ort` crate's pre-built ONNX Runtime binary requires glibc >= 2.38. The original `rust:bookworm` (glibc 2.36) causes linker errors.
- **Inference stack**: `/api/simulate` tries **PyTorch FNO sidecar** first only if `KLIMA_FNO_URL` is set (non-empty). Default `./scripts/run.sh` leaves it unset for fast dev (ONNX if `KLIMA_MODEL_PATH` loads, else mock). Use **`./scripts/run.sh dev-infer`** to start `klima-infer` and set `KLIMA_FNO_URL=http://klima-infer:8000`. Sidecar: `training/infer_server/` + `best_model.pt` / `norm_params.json` — see `training/infer_server/README.md`. Local FNO is not exportable to ONNX (`fft_rfftn`); ONNX remains useful for mock/integration graphs.
- **Cesium Ion token**: Optional. Put `CESIUM_ION_TOKEN=...` in a root **`.env`** file (sourced by `./scripts/run.sh`) or export in the shell — forwarded to the frontend compose as `VITE_CESIUM_ION_TOKEN`. Without it: OSM imagery on a dark globe, no starfield.
- **Frontend proxy**: The frontend dev server proxies `/api/*` to `http://klima-back:3000` via Docker networking. For local (non-Docker) testing, use `VITE_API_BASE_URL=http://localhost:3000`.

### Database schema

Versioned SQL migrations live in `back/migrations/` and run at startup via `sqlx::migrate!` (embedded at compile time). Add new files as `back/migrations/<timestamp>_<name>.sql`; existing databases pick up only pending revisions.

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
