#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$PROJECT_ROOT/target-current}"
FALLBACK_TARGET_DIR="$PROJECT_ROOT/target"

cd "$PROJECT_ROOT"

if command -v cargo >/dev/null 2>&1; then
  CARGO_TARGET_DIR="$TARGET_DIR" exec cargo run --quiet --release -- "$@"
fi

if command -v cargo.exe >/dev/null 2>&1; then
  CARGO_TARGET_DIR="$TARGET_DIR" exec cargo.exe run --quiet --release -- "$@"
fi

if [[ -x "$TARGET_DIR/release/document-templating-system.exe" ]]; then
  exec "$TARGET_DIR/release/document-templating-system.exe" "$@"
fi

if [[ -x "$TARGET_DIR/release/document-templating-system" ]]; then
  exec "$TARGET_DIR/release/document-templating-system" "$@"
fi

if [[ -x "$FALLBACK_TARGET_DIR/release/document-templating-system.exe" ]]; then
  exec "$FALLBACK_TARGET_DIR/release/document-templating-system.exe" "$@"
fi

if [[ -x "$FALLBACK_TARGET_DIR/release/document-templating-system" ]]; then
  exec "$FALLBACK_TARGET_DIR/release/document-templating-system" "$@"
fi

printf 'error: document-templating-system binary is missing and Cargo was not found.\n' >&2
exit 1
