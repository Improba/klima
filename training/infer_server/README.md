# Klima FNO inference sidecar

FastAPI service that loads **`best_model.pt`** (Local FNO) and applies the same z-score normalisation as training (`norm_params.json`). The Rust API calls it over HTTP when `KLIMA_FNO_URL` is set.

## Docker (with dev stack)

Service **`klima-infer`** dans `back/docker/docker-compose.dev.yml` (profil Compose **`infer`**). Démarrage : **`./scripts/run.sh dev-infer`** (définit aussi `KLIMA_FNO_URL` côté backend). Mounts :

- Monorepo root (read-only) at `/app/repo` — `PYTHONPATH=/app/repo` so `training.src.model.local_fno` imports work.
- `training/checkpoints` at `/checkpoints` — expects `best_model.pt` and `norm_params.json` (or `norm_params.json` in the **same directory as** the `.pt` if `KLIMA_FNO_NORM` is absent or wrong). Without valid norm params, **`POST /predict` returns 503** (inputs must match training z-score).

Host port **8001** → container **8000**.

## Endpoints

- `GET /health` — `{ "status": "ok" | "no_model" | "no_norm", "device": "..." }` (`no_norm` = checkpoint chargé mais pas de `norm_params.json` → `/predict` en 503)
- `POST /predict` — body: binary format `KLM1` + `ndim` + `shape[]` + raw `float32` row-major (same layout as `ndarray` C-contiguous). Response: same header + output tensor `[1,4,nx,ny,nz]`.

## Env

| Variable | Default | Role |
|----------|---------|------|
| `KLIMA_FNO_CHECKPOINT` | `/checkpoints/best_model.pt` | PyTorch checkpoint |
| `KLIMA_FNO_NORM` | `/checkpoints/norm_params.json` | Normalisation JSON |

## Local run (without Docker)

From monorepo root, with checkpoint under `training/checkpoints/`:

```bash
pip install -r training/infer_server/requirements.txt
export PYTHONPATH=.
export KLIMA_FNO_CHECKPOINT=training/checkpoints/best_model.pt
export KLIMA_FNO_NORM=training/checkpoints/norm_params.json
cd training/infer_server && uvicorn main:app --host 127.0.0.1 --port 8000
```

Set `KLIMA_FNO_URL=http://127.0.0.1:8000` for `klima-back`.

## Image size

`requirements.txt` pulls the default **CUDA** PyTorch wheel (~GB). For CPU-only machines you can switch to the CPU index URL in `requirements.txt` to shrink the image.
