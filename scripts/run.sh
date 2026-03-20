#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
BACK_COMPOSE="$ROOT_DIR/back/docker/docker-compose.dev.yml"
FRONT_COMPOSE="$ROOT_DIR/front/docker/docker-compose.dev.yml"
BACK_DOCKER_DIR="$ROOT_DIR/back/docker"
BACK_ENV_FILE="$BACK_DOCKER_DIR/.env"

RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m'

log() { echo -e "${CYAN}[klima]${NC} $*"; }
ok()  { echo -e "${GREEN}[klima]${NC} $*"; }
err() { echo -e "${RED}[klima]${NC} $*" >&2; }

# Root `.env` → shell (CESIUM_ION_TOKEN, etc.) pour interpolation compose front/back.
load_root_env() {
  if [[ -f "$ROOT_DIR/.env" ]]; then
    set -a
    # shellcheck source=/dev/null
    source "$ROOT_DIR/.env"
    set +a
  fi
}

print_usage() {
  echo "Usage: $0 [command] [options...]"
  echo ""
  echo "  (no args)       Start dev stack: DB + backend + frontend (sans sidecar FNO)"
  echo "  dev, up         Rapide : ONNX puis mock, pas d’appel HTTP vers klima-infer"
  echo "  dev-infer       + sidecar PyTorch FNO (http://localhost:8001)"
  echo "  down [opts]     Stop stacks; options passed to docker compose down (e.g. -v)"
  echo "  logs [opts]     Follow logs (backend + infer si actif, + frontend)"
  echo "  restart         down puis dev (sans infer)"
  echo "  restart-infer   down puis dev-infer"
  echo ""
  echo "Fichiers optionnels :"
  echo "  $ROOT_DIR/.env              → CESIUM_ION_TOKEN, etc."
  echo "  $BACK_ENV_FILE   → surcharges (voir back/docker/.env.example)"
}

if ! command -v docker &>/dev/null; then
  err "Docker is not installed. Please install Docker first."
  exit 1
fi

if ! docker info &>/dev/null; then
  err "Docker daemon is not running."
  exit 1
fi

if [[ $# -eq 0 ]]; then
  CMD=dev
  EXTRA=()
else
  CMD="$1"
  shift
  EXTRA=("$@")
fi

case "$CMD" in
  dev|up)
    load_root_env
    _BACK_ENV=()
    [[ -f "$BACK_ENV_FILE" ]] && _BACK_ENV=(--env-file "$BACK_ENV_FILE")

    log "Creating shared Docker network..."
    docker network create klima-net 2>/dev/null || true

    log "Starting Klima backend (Rust/Axum)..."
    docker compose -f "$BACK_COMPOSE" "${_BACK_ENV[@]}" up -d --build

    log "Starting Klima frontend (Vue/Quasar/CesiumJS)..."
    docker compose -f "$FRONT_COMPOSE" up -d --build

    echo ""
    ok "Klima dev environment is running!"
    ok "  Backend  → http://localhost:3000"
    ok "  Frontend → http://localhost:9000"
    echo ""
    log "Useful commands:"
    echo "  $0 dev-infer                         # + sidecar FNO (http://localhost:8001)"
    echo "  docker exec -it klima-back  bash     # Shell into backend"
    echo "  docker exec -it klima-front bash     # Shell into frontend"
    echo "  $0 logs                              # Tail logs"
    echo "  $0 down                              # Stop everything"
    ;;

  dev-infer|up-infer)
    load_root_env
    export COMPOSE_PROFILES="${COMPOSE_PROFILES:-infer}"
    export KLIMA_FNO_URL="${KLIMA_FNO_URL:-http://klima-infer:8000}"
    _BACK_ENV=()
    [[ -f "$BACK_ENV_FILE" ]] && _BACK_ENV=(--env-file "$BACK_ENV_FILE")

    log "Creating shared Docker network..."
    docker network create klima-net 2>/dev/null || true

    log "Starting Klima backend + FNO sidecar (profile infer)..."
    docker compose -f "$BACK_COMPOSE" --profile infer "${_BACK_ENV[@]}" up -d --build

    log "Starting Klima frontend (Vue/Quasar/CesiumJS)..."
    docker compose -f "$FRONT_COMPOSE" up -d --build

    echo ""
    ok "Klima dev environment is running (with PyTorch FNO sidecar)!"
    ok "  Backend   → http://localhost:3000"
    ok "  Frontend  → http://localhost:9000"
    ok "  FNO infer → http://localhost:8001  (klima-infer:8000 dans le réseau Docker)"
    echo ""
    log "Checkpoints : training/checkpoints/best_model.pt + norm_params.json"
    ;;

  down)
    load_root_env
    _BACK_ENV=()
    [[ -f "$BACK_ENV_FILE" ]] && _BACK_ENV=(--env-file "$BACK_ENV_FILE")
    log "Stopping all Klima containers..."
    docker compose -f "$FRONT_COMPOSE" down "${EXTRA[@]}"
    docker compose -f "$BACK_COMPOSE" --profile infer "${_BACK_ENV[@]}" down "${EXTRA[@]}"
    ok "All containers stopped."
    ;;

  logs)
    load_root_env
    _BACK_ENV=()
    [[ -f "$BACK_ENV_FILE" ]] && _BACK_ENV=(--env-file "$BACK_ENV_FILE")
    docker compose -f "$BACK_COMPOSE" --profile infer "${_BACK_ENV[@]}" logs -f "${EXTRA[@]}" &
    docker compose -f "$FRONT_COMPOSE" logs -f "${EXTRA[@]}" &
    wait
    ;;

  restart)
    "$SCRIPT_DIR/run.sh" down "${EXTRA[@]}"
    "$SCRIPT_DIR/run.sh" dev
    ;;

  restart-infer)
    "$SCRIPT_DIR/run.sh" down "${EXTRA[@]}"
    "$SCRIPT_DIR/run.sh" dev-infer
    ;;

  -h|--help|help)
    print_usage
    exit 0
    ;;

  *)
    err "Unknown command: $CMD"
    print_usage
    exit 1
    ;;
esac
