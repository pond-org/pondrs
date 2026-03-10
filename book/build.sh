#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ASSETS_DIR="$SCRIPT_DIR/src/assets"

mkdir -p "$ASSETS_DIR"

echo "Building examples..."
cargo build --examples --manifest-path "$ROOT_DIR/Cargo.toml"

echo "Generating static viz pages..."

cargo run --manifest-path "$ROOT_DIR/Cargo.toml" --example weather_app -- \
    viz --export "$ASSETS_DIR/weather_viz.html"

cargo run --manifest-path "$ROOT_DIR/Cargo.toml" --example sales_app -- \
    viz --export "$ASSETS_DIR/sales_viz.html"

cargo run --manifest-path "$ROOT_DIR/Cargo.toml" --example ident_app -- \
    --catalog-path "$ROOT_DIR/examples/ident_data/catalog.yml" \
    --params-path "$ROOT_DIR/examples/ident_data/params.yml" \
    viz --export "$ASSETS_DIR/ident_viz.html"

cargo run --manifest-path "$ROOT_DIR/Cargo.toml" --example register_example -- \
    viz --export "$ASSETS_DIR/register_viz.html"

echo "Building book..."
mdbook build "$SCRIPT_DIR"

echo "Done. Output in $SCRIPT_DIR/book/"
