#!/usr/bin/env bash
# RemitPulse: build optimized artifacts, deploy, and atomically initialize constructors.
# Mainnet preflight: complete docs/mainnet-deployment-checklist.md before deploying.
# Never commit private keys or other secrets; supply them only via environment/CLI identity.
set -euo pipefail
cd "$(dirname "$0")"
network=${1:-testnet}
component=${2:-core}
case "$network" in testnet|mainnet) ;; *) echo "Network must be testnet or mainnet (got '$network')" >&2; exit 2;; esac
case "$component" in core|router|webauthn|account|subsidy|airdrop|nft) ;; *) echo "Unknown component '$component' (expected one of: core, router, webauthn, account, subsidy, airdrop, nft)" >&2; exit 2;; esac
case "$network" in
  testnet) expected_passphrase='Test SDF Network ; September 2015';;
  mainnet) expected_passphrase='Public Global Stellar Network ; September 2015';;
esac
: "${STELLAR_NETWORK_PASSPHRASE:?Set the expected Stellar network passphrase}"
if [[ "$STELLAR_NETWORK_PASSPHRASE" != "$expected_passphrase" ]]; then
  echo "STELLAR_NETWORK_PASSPHRASE does not match $network" >&2
  exit 2
fi
if [[ "$network" == mainnet && "${REMITPULSE_ALLOW_MAINNET:-}" != yes ]]; then
  echo 'Mainnet deployment requires REMITPULSE_ALLOW_MAINNET=yes' >&2
  exit 2
fi
: "${SOURCE_ACCOUNT:?Set SOURCE_ACCOUNT to a funded CLI identity}"
CLI=${STELLAR_CLI:-stellar} # Set to soroban for the legacy soroban-cli binary.
command -v "$CLI" >/dev/null
# Validate all required constructor inputs before spending network fees.
case "$component" in
  core|account) : "${ACCOUNT_CONFIG:?Set ACCOUNT_CONFIG to the Config JSON object}";;
esac
case "$component" in core|subsidy|airdrop) : "${ADMIN:?Set ADMIN to the administrator address or CLI identity}";; esac
case "$component" in
  airdrop) : "${ROOT_HASH:?Set ROOT_HASH to the airdrop Merkle root}" "${TOKEN:?Set TOKEN to the token contract address}" "${FUNDER:?Set FUNDER to the funding address or CLI identity}";;
  nft) : "${OWNER:?Set OWNER to the NFT owner address or CLI identity}" "${MAX_SUPPLY:?Set MAX_SUPPLY to the maximum token supply}" "${METADATA:?Set METADATA JSON}";;
esac
if ! python3 scripts/validate_deployment.py "$component"; then
  echo "Deployment preflight failed for component '$component'" >&2
  exit 2
fi
# Stellar CLI builds optimized WASM by default. Legacy CLI requires explicit optimize.
STELLAR_CLI="$CLI" ./scripts/build.sh
if [[ "${CLI##*/}" == soroban ]]; then
  # SC2035: the glob is intentionally relative and unquoted so it expands to the
  # built WASM artifacts; quoting would pass a literal pattern to the loop.
  # shellcheck disable=SC2035
  for wasm in target/wasm32v1-none/release/remitpulse_*.wasm; do
    [[ "$wasm" == *.optimized.wasm ]] && continue
    "$CLI" contract optimize --wasm "$wasm" --wasm-out "${wasm%.wasm}.optimized.wasm"
  done
fi
mkdir -p deployments
record="deployments/${network}-$(date -u +%Y%m%dT%H%M%SZ)-$$.env"
: > "$record"
printf 'REMITPULSE_NETWORK=%s\nREMITPULSE_NETWORK_PASSPHRASE=%s\nREMITPULSE_COMPONENT=%s\nREMITPULSE_DEPLOYED_AT=%s\n' \
  "$network" "$STELLAR_NETWORK_PASSPHRASE" "$component" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "$record"
deploy() {
  local name=$1 id wasm
  shift
  wasm="target/wasm32v1-none/release/remitpulse_${name}.wasm"
  [[ "${CLI##*/}" == soroban ]] && wasm="${wasm%.wasm}.optimized.wasm"
  if (( $# )); then
    # SC2068: "$@" is intentionally forwarded verbatim as constructor args to the
    # CLI; each argument is already quoted by the caller, so no re-splitting occurs.
    # shellcheck disable=SC2068
    id=$("$CLI" contract deploy --wasm "$wasm" --source-account "$SOURCE_ACCOUNT" --network "$network" -- "$@")
  else
    id=$("$CLI" contract deploy --wasm "$wasm" --source-account "$SOURCE_ACCOUNT" --network "$network")
  fi
  printf 'REMITPULSE_%s=%s\n' "${name^^}" "$id" | tee -a "$record"
}
case "$component" in
  core) deploy router; deploy webauthn; deploy account --config "$ACCOUNT_CONFIG"; deploy subsidy --admin "$ADMIN";;
  account) deploy account --config "$ACCOUNT_CONFIG";;
  subsidy) deploy subsidy --admin "$ADMIN";;
  airdrop) deploy airdrop --root_hash "$ROOT_HASH" --token "$TOKEN" --admin "$ADMIN" --funder "$FUNDER";;
  nft) deploy nft --owner "$OWNER" --max_supply "$MAX_SUPPLY" --metadata "$METADATA";;
  *) deploy "$component";;
esac
printf 'Deployment IDs saved to %s\n' "$record"
