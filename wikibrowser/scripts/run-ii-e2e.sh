#!/usr/bin/env bash
set -euo pipefail

if [ -f .env.e2e.local ]; then
  set -a
  . ./.env.e2e.local
  set +a
fi

playwright test --config playwright.config.ts "$@"
