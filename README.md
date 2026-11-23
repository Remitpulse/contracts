# RemitPulse Stellar Testnet treasury

This repository provides read-only settlement checks for the casino's native XLM treasury. The milestone uses Stellar Classic payment operations for native XLM and has no Soroban contract to deploy. Simulated wagers settle in the casino backend's SQLite ledger.

## Use

With Node.js 24, run `npm ci` and `npm test`. To inspect the Testnet treasury, run `npm run treasury -- balance <treasury G-address>`. To verify a casino payment, run `npm run treasury -- receipt <transaction hash> <treasury G-address>`.

The receipt command requires a successful, single-operation native XLM payment involving the treasury. It emits the ledger, payment direction, exact XLM and integer stroops. The explorer endpoint is fixed to Stellar Testnet. These tools require public addresses only.
