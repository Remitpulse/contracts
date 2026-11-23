import assert from 'node:assert/strict'
import { test } from 'node:test'
import { verifyFootballSource } from './football-source.mjs'

const receipt = {
  event: 'football-data:123', state: 'Won', selection: 1,
  price: 320, stake: '10000001', payout: '32000003', score: '2–2',
}
const match = {
  id: 123, status: 'FINISHED',
  score: { duration: 'PENALTY_SHOOTOUT', fullTime: { home: 4, away: 3 }, regularTime: { home: 2, away: 2 } },
}

test('checks the same provider match and uses regulation time', async () => {
  assert.equal(await verifyFootballSource(receipt, 'test-token', async (url, options) => {
    assert.equal(url, 'https://api.football-data.org/v4/matches/123')
    assert.equal(options.headers['X-Auth-Token'], 'test-token')
    assert.equal(options.redirect, 'error')
    return Response.json(match)
  }), true)
  assert.equal(await verifyFootballSource(receipt, 'test-token', async () => Response.json({
    ...match, score: { ...match.score, regularTime: { home: 2, away: 1 } },
  })), false)
  await assert.rejects(verifyFootballSource(receipt, 'test-token', async () => Response.json({
    ...match, id: 456,
  })), /requested match/)
})
