# Contributing to the Stellar Testnet treasury toolkit

The treasury CLI verifies native XLM transaction records and checks accounting
totals in integer stroops. It runs from this repository independently of the
RemitPulse application. Keep new treasury examples independent of the casino
backend and usable with public or synthetic data.

## Local setup

Use Node.js 24 or later.

```sh
npm ci
npm test
npm run treasury -- reconcile-file examples/testnet-reconciliation.json
```

The sample reconciliation runs offline and needs no wallets or credentials.
See the README for verifying a public Testnet transaction against Horizon.

## Submitting changes

- Choose a scoped issue or open one describing the behavior and how to verify it.
- Keep amount calculations exact with integer stroops. Add or update a focused
  test when changing verification or reconciliation behavior.
- Use synthetic records or public Testnet transaction data in tests. Keep
  signing keys, private ledger exports, and customer data out of issues,
  commits, and pull requests.
- Open a pull request against the repository default branch. Link the issue
  and include what changed, the test commands you ran, and any compatibility
  considerations. Maintainers review the behavior, test coverage, and CI
  results before merging.

For security reports, follow [SECURITY.md](SECURITY.md).
