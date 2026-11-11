#!/usr/bin/env bash
# Keep contract compilation on the tested toolchain, including inside CLI subprocesses.
set -euo pipefail
cd "$(dirname "$0")/.."
toolchain=$(sed -n 's/^channel = "\([^"]*\)"$/\1/p' rust-toolchain.toml)
: "${toolchain:?Missing channel in rust-toolchain.toml}"
# An inherited RUSTUP_TOOLCHAIN otherwise overrides rust-toolchain.toml.
export RUSTUP_TOOLCHAIN="$toolchain"

# Optional output directory: pass as the first argument or via BUILD_OUT_DIR.
# When unset, the CLI default behavior is preserved (make build, deploy.sh).
out_dir="${1:-${BUILD_OUT_DIR:-}}"

if [ -n "$out_dir" ]; then
  mkdir -p "$out_dir"
  "${STELLAR_CLI:-stellar}" contract build --out-dir "$out_dir"
else
  "${STELLAR_CLI:-stellar}" contract build
fi

# Verify the build produced predictably named WASM outputs for the build target.
# Artifact names and checksums depend on these files existing at this path.
target_dir="${out_dir:-target/wasm32v1-none/release}"
if [ ! -d "$target_dir" ]; then
  echo "error: expected build target output directory '$target_dir' is missing" >&2
  exit 1
fi

shopt -s nullglob
wasm_files=("$target_dir"/remitpulse_*.wasm)
shopt -u nullglob

if [ "${#wasm_files[@]}" -eq 0 ]; then
  echo "error: no remitpulse_*.wasm outputs found in '$target_dir'" >&2
  exit 1
fi

echo "Built WASM artifacts:"
for wasm in "${wasm_files[@]}"; do
  echo "  $wasm"
done
