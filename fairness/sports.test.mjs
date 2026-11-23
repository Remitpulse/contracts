import assert from 'node:assert/strict'
import { test } from 'node:test'
import { verifySportsPayout } from './sports.mjs'

const ticket = { state: 'Won', price: 235, stake: '10000001', payout: '23500002' }

test('checks that winning sports payouts round down to the nearest stroop', () => {
  assert.equal(verifySportsPayout(ticket), true)
  assert.equal(verifySportsPayout({ ...ticket, payout: '23500003' }), false)
  assert.equal(verifySportsPayout({ ...ticket, stake: '1.0000001' }), false)
})

test('void tickets refund the exact stake and losing tickets return zero', () => {
  assert.equal(verifySportsPayout({ ...ticket, state: 'Void', payout: ticket.stake }), true)
  assert.equal(verifySportsPayout({ ...ticket, state: 'Void' }), false)
  assert.equal(verifySportsPayout({ ...ticket, state: 'Lost', payout: '0' }), true)
  assert.equal(verifySportsPayout({ ...ticket, state: 'Open', payout: '0' }), false)
})
