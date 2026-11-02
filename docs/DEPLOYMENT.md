# Deployment

Deployments are manual; CI and tests never contact Stellar networks. Set
`SOURCE_ACCOUNT`, required constructor variables, and
`STELLAR_NETWORK_PASSPHRASE`. Testnet requires `Test SDF Network ; September
2015`. Mainnet also requires the public-network passphrase and
`REMITPULSE_ALLOW_MAINNET=yes`.

```sh
./deploy.sh testnet core
```

The script validates constructor inputs, builds WASM, and records selected
network, component, timestamp, and contract IDs in a gitignored deployment
manifest. Component deployments are separate transactions. After deployment,
read `account config` and `subsidy admin` with the Stellar CLI on the same
network and compare locally built WASM hashes with reviewed CI artifacts. Never
store secrets in manifests or shell history.
