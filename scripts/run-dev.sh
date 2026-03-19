#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m'

log() { echo -e "${CYAN}[klima]${NC} $*"; }
ok()  { echo -e "${GREEN}[klima]${NC} $*"; }
err() { echo -e "${RED}[klima]${NC} $*" >&2; }

if ! command -v docker &>/dev/null; then
  err "Docker is not installed. Please install Docker first."
  exit 1
fi

if ! docker info &>/dev/null; then
  err "Docker daemon is not running."
  exit 1
fi

log "Creating shared Docker network..."
docker network create klima-net 2>/dev/null || true

ACTION="${1:-up}"

case "$ACTION" in
  up)
    log "Starting Klima backend (Rust/Axum)..."
    docker compose -f "$ROOT_DIR/back/docker/docker-compose.dev.yml" up -d --build

    log "Starting Klima frontend (Vue/Quasar/CesiumJS)..."
    docker compose -f "$ROOT_DIR/front/docker/docker-compose.dev.yml" up -d --build

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
    docker compose -f "$ROOT_DIR/front/docker/docker-compose.dev.yml" down
    docker compose -f "$ROOT_DIR/back/docker/docker-compose.dev.yml" down
    ok "All containers stopped."
    ;;

  logs)
    docker compose -f "$ROOT_DIR/back/docker/docker-compose.dev.yml" logs -f &
    docker compose -f "$ROOT_DIR/front/docker/docker-compose.dev.yml" logs -f &
    wait
    ;;

  restart)
    "$0" down
    "$0" up
    ;;

  *)
    echo "Usage: $0 {up|down|logs|restart}"
    exit 1
    ;;
esac
