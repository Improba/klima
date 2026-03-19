#!/usr/bin/env python3
"""Check all Python files in training/src for syntax errors."""
import ast
import os
import sys


def main():
    errors = []
    count = 0
    for root, dirs, files in os.walk("training/src"):
        dirs[:] = [d for d in dirs if d != "__pycache__"]
        for f in files:
            if f.endswith(".py"):
                path = os.path.join(root, f)
                try:
                    with open(path) as fh:
                        ast.parse(fh.read())
                    count += 1
                except SyntaxError as e:
                    errors.append(f"{path}: {e}")

    if errors:
        for e in errors:
            print(f"SYNTAX ERROR: {e}", file=sys.stderr)
        sys.exit(1)

    print(f"{count} Python files parsed OK")


if __name__ == "__main__":
    main()
