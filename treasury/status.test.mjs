import assert from 'node:assert/strict'
import { test } from 'node:test'
import { lookupTransfer } from './status.mjs'

const hash = 'a'.repeat(64)
const horizon = call => ({ transactions: () => ({ transaction: id => {
  assert.equal(id, hash)
  return { call }
} }) })

test('distinguishes confirmed, failed and unindexed transfers', async () => {
  for (const [successful, status] of [[true, 'confirmed'], [false, 'failed']]) {
    assert.deepEqual(await lookupTransfer(horizon(async () => ({ successful, ledger_attr: 123 })), hash),
      { hash, status, ledger: 123 })
  }
  assert.deepEqual(await lookupTransfer(horizon(async () => {
    throw { response: { status: 404 } }
  }), hash), { hash, status: 'not-found', ledger: null })
})

test('propagates service outages instead of reporting missing transfers', async () => {
  const outage = new Error('timeout')
  await assert.rejects(lookupTransfer(horizon(async () => { throw outage }), hash), outage)
})

test('requires a recorded outcome and ledger before reporting a terminal status', async () => {
  for (const transaction of [
    { ledger_attr: 42 }, { successful: false }, { successful: false, ledger_attr: 0 },
  ]) {
    await assert.rejects(lookupTransfer(horizon(async () => transaction), hash), /incomplete transfer receipt/)
  }
})
