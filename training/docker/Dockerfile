FROM pytorch/pytorch:2.5.1-cuda12.4-cudnn9-runtime

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY . .

# Imports use package `training.src` (monorepo layout). docker-compose.yml mounts the repo root and sets PYTHONPATH=/app.
