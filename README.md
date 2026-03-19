# klima

Projet climat.

## Installation

```bash
python -m venv .venv
source .venv/bin/activate
pip install -e .
```

## Utilisation

```bash
klima
```

## Tests

```bash
pip install pytest
pytest
```

## Structure du projet

```
klima/
├── src/
│   └── klima/          # Code source
│       ├── __init__.py
│       └── main.py
├── tests/              # Tests
│   └── test_main.py
├── pyproject.toml      # Configuration du projet
└── README.md
```
