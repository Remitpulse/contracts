import assert from 'node:assert/strict'
import { test } from 'node:test'
import { nativePayments, stroops } from './native.mjs'

test('preserves amounts above the JS safe integer limit and rejects fractions of a stroop', () => {
  assert.equal(stroops('99999999999.0000001'), '999999999990000001')
  assert.throws(() => stroops('0.00000001'))
})

test('filters issued assets and unrelated native transfers from treasury receipts', () => {
  const base = { type: 'payment', asset_type: 'native', from: 'PLAYER', to: 'TREASURY', amount: '0.0000001' }
  const result = nativePayments([base, { ...base, asset_type: 'credit_alphanum4' }, { ...base, to: 'OTHER' }], 'TREASURY')
  assert.equal(result.length, 1)
  assert.equal(result[0].stroops, '1')
  assert.equal(result[0].direction, 'inbound')
})

test('excludes treasury self-payments from reconciliation receipts', () => {
  const operation = {
    type: 'payment', asset_type: 'native', from: 'TREASURY', to: 'TREASURY', amount: '1',
  }
  assert.deepEqual(nativePayments([operation], 'TREASURY'), [])
})
