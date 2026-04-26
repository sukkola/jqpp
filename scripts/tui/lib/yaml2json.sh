#!/usr/bin/env bash
# yaml2json.sh — Convert a YAML file to JSON on stdout.
# Usage: yaml2json.sh <file.yaml>
# Requires Python 3 (yaml stdlib available on all target platforms).

set -euo pipefail

if [ $# -ne 1 ]; then
  echo "Usage: yaml2json.sh <file.yaml>" >&2
  exit 1
fi

FILE="$1"

if [ ! -f "$FILE" ]; then
  echo "ERROR: file not found: $FILE" >&2
  exit 1
fi

python3 - "$FILE" <<'EOF'
import sys, json, yaml
with open(sys.argv[1]) as f:
    print(json.dumps(yaml.safe_load(f)))
EOF
