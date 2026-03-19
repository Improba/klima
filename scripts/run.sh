#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
BACK_COMPOSE="$ROOT_DIR/back/docker/docker-compose.dev.yml"
FRONT_COMPOSE="$ROOT_DIR/front/docker/docker-compose.dev.yml"

RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m'

log() { echo -e "${CYAN}[klima]${NC} $*"; }
ok()  { echo -e "${GREEN}[klima]${NC} $*"; }
err() { echo -e "${RED}[klima]${NC} $*" >&2; }

print_usage() {
  echo "Usage: $0 [command] [options...]"
  echo ""
  echo "  (no args)     Start dev stack (backend + DB + frontend)"
  echo "  dev, up       Same as above"
  echo "  down [opts]   Stop stacks; options are passed to docker compose down (e.g. -v)"
  echo "  logs [opts]   Follow logs from backend and frontend"
  echo "  restart       down then dev (extra args after restart are passed to down)"
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
    log "Creating shared Docker network..."
    docker network create klima-net 2>/dev/null || true

    log "Starting Klima backend (Rust/Axum)..."
    docker compose -f "$BACK_COMPOSE" up -d --build

    log "Starting Klima frontend (Vue/Quasar/CesiumJS)..."
    docker compose -f "$FRONT_COMPOSE" up -d --build

    echo ""
    ok "Klima dev environment is running!"
    ok "  Backend  → http://localhost:3000"
    ok "  Frontend → http://localhost:9000"
    echo ""
    log "Useful commands:"
    echo "  docker exec -it klima-back  bash     # Shell into backend"
    echo "  docker exec -it klima-front bash     # Shell into frontend"
    echo "  $0 logs                              # Tail logs"
    echo "  $0 down                              # Stop everything"
    ;;

  down)
    log "Stopping all Klima containers..."
    docker compose -f "$FRONT_COMPOSE" down "${EXTRA[@]}"
    docker compose -f "$BACK_COMPOSE" down "${EXTRA[@]}"
    ok "All containers stopped."
    ;;

  logs)
    docker compose -f "$BACK_COMPOSE" logs -f "${EXTRA[@]}" &
    docker compose -f "$FRONT_COMPOSE" logs -f "${EXTRA[@]}" &
    wait
    ;;

  restart)
    "$SCRIPT_DIR/run.sh" down "${EXTRA[@]}"
    "$SCRIPT_DIR/run.sh" dev
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
