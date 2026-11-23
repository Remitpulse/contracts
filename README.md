# RemitPulse Stellar Testnet treasury

This repository contains read-only tools for verifying native XLM wallet activity. The casino and sportsbook settle simulated balances in the [backend ledger](https://github.com/Remitpulse/backend/tree/stellar-casino-milestone). Native deposits and withdrawals use Stellar Classic payment operations.

## Install and test

Use Node.js 24.

```sh
npm ci
npm test
```

## Verify treasury activity

Run `npm run treasury -- balance <treasury G-address>` to read the treasury balance. Run `npm run treasury -- receipt <transaction hash> <treasury G-address>` to verify a wallet payment.

The receipt command requires a successful transaction with one native XLM payment involving the treasury. It reports the transaction hash, ledger, payment direction, exact XLM and integer stroops. The endpoint is fixed to Stellar Testnet. Use public addresses and transaction hashes. Keep signing secrets in the backend configuration.

## Verify a Dice receipt

In RemitPulse, open Activity, expand **Dice proof**, and save the contents of
**Proof data** as `dice-receipt.json`. On a separate machine if desired, run:

```sh
npm run fairness:verify -- dice-receipt.json
```

The verifier recomputes the commitment from the revealed server seed, derives
the roll from both seeds, and calculates the payout using integer stroops.
To check that the commitment was present before the wager, compare the receipt
commitment with the value captured before submitting the client seed. Pass that
value as a second argument to make the command check it.
