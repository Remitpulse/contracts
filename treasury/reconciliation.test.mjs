import assert from 'node:assert/strict'
import { test } from 'node:test'
import { verifyReconciliation } from './reconciliation.mjs'

const summary = {
  network: 'stellar-testnet',
  availableStroops: '9007199254740993',
  reservedStroops: '2',
  liabilitiesStroops: '9007199254740995',
  treasuryStroops: '9007199254740994',
  grossDifferenceStroops: '-1',
}

test('checks a negative treasury difference and exact liabilities above safe integer range', () => {
  assert.equal(verifyReconciliation(summary).treasuryCompared, true)
  assert.throws(() => verifyReconciliation({ ...summary, reservedStroops: '1' }), /liabilities/)
  assert.throws(() => verifyReconciliation({ ...summary, grossDifferenceStroops: '0' }), /difference/)
  assert.throws(() => verifyReconciliation({ ...summary, network: 'stellar-mainnet' }), /Testnet/)
})

test('permits an unavailable treasury only without a recorded difference', () => {
  assert.equal(verifyReconciliation({ ...summary, treasuryStroops: null, grossDifferenceStroops: null }).treasuryCompared, false)
  assert.throws(() => verifyReconciliation({ ...summary, treasuryStroops: null }), /requires/)
})
