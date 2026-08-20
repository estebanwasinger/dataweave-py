#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

pytest_args=("$@")

run_python_suite() {
  local backend="$1"
  echo "=== DataWeave tests: $backend ==="
  if ((${#pytest_args[@]})); then
    DWPY_TEST_BACKEND="$backend" \
      UV_CACHE_DIR=.uv-cache \
      uv run --extra dev pytest "${pytest_args[@]}"
  else
    DWPY_TEST_BACKEND="$backend" \
      UV_CACHE_DIR=.uv-cache \
      uv run --extra dev pytest
  fi
}

overall_status=0

run_python_suite python || overall_status=1
run_python_suite rust || overall_status=1

rust_bin="$(dirname "$(rustup which rustc)")"
echo "=== Building WASM package ==="
PATH="$rust_bin:$PATH" \
  npm --prefix packages/dataweave-wasm run build

run_python_suite wasm || overall_status=1

exit "$overall_status"
