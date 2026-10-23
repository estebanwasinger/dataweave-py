#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# Prefer rustup's compiler: a Homebrew rustc may lack the WASM target.
if command -v rustup >/dev/null 2>&1; then
  rust_bin="$(dirname "$(rustup which rustc)")"
  export PATH="$rust_bin:$PATH"
fi

# wasm-pack installs the matching wasm-bindgen test runner and selects Node.
# Additional cargo test arguments are forwarded; put filters after `--`.
wasm-pack test --node crates/dwpy-wasm "$@"
