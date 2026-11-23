import assert from 'node:assert/strict'
import { test } from 'node:test'
import { verifyDiceReceipt } from './dice.mjs'

const receipt = {
  version: 1,
  commitment: '66687aadf862bd776c8fc18b8e9f8e20089714856ee233b3902a591d0d5f2925',
  serverSeed: '00'.repeat(32),
  clientSeed: '42'.repeat(32),
  roll: 1335,
  target: 50,
  stake: '10000001',
  payout: '19800001',
}

test('verifies a known Dice receipt with exact integer payout', () => {
  assert.equal(verifyDiceReceipt(receipt), true)
})

test('rejects changes to commitment, round inputs, roll or payout', () => {
  for (const alteration of [
    { commitment: '0'.repeat(64) }, { clientSeed: '01'.repeat(32) },
    { roll: 1336 }, { target: 51 }, { stake: '10000002' }, { payout: '19800002' },
  ]) {
    assert.equal(verifyDiceReceipt({ ...receipt, ...alteration }), false)
  }
})
