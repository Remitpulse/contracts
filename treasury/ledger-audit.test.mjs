import assert from 'node:assert/strict'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { DatabaseSync } from 'node:sqlite'
import { test } from 'node:test'
import { auditLedger } from './ledger-audit.mjs'

test('audits exact liabilities and can read snapshots from before the failed-deposit migration', () => {
  const directory = mkdtempSync(join(tmpdir(), 'rp-ledger-audit-'))
  const file = join(directory, 'ledger.sqlite')
  const db = new DatabaseSync(file)
  try {
    db.exec(`
      CREATE TABLE player (id TEXT PRIMARY KEY, testnet INTEGER);
      INSERT INTO player VALUES ('one', 9007199254740992), ('two', 1);
      CREATE TABLE withdrawal (pending INTEGER, amount TEXT);
      INSERT INTO withdrawal VALUES (1, '3'), (0, '7');
      CREATE TABLE deposit (hash TEXT, credited INTEGER);
      CREATE TABLE signed_deposit (hash TEXT);
      INSERT INTO deposit VALUES ('failed', 0), ('unsigned', 0), ('credited', 1);
      INSERT INTO signed_deposit VALUES ('failed'), ('credited');
    `)
    const { balanceFingerprint } = auditLedger(file)
    assert.match(balanceFingerprint, /^[a-f0-9]{64}$/)
    const expected = {
      balanceFingerprint, players: 2, availableStroops: '9007199254740993', reservedStroops: '3',
      liabilitiesStroops: '9007199254740996', pendingDeposits: 1, pendingWithdrawals: 1,
    }
    assert.deepEqual(auditLedger(file), expected)
    db.exec("CREATE TABLE failed_deposit (hash TEXT); INSERT INTO failed_deposit VALUES ('failed')")
    assert.deepEqual(auditLedger(file), { ...expected, pendingDeposits: 0 })
    db.exec("UPDATE player SET testnet = testnet + CASE id WHEN 'one' THEN 1 ELSE -1 END")
    const changed = auditLedger(file)
    assert.equal(changed.liabilitiesStroops, expected.liabilitiesStroops)
    assert.notEqual(changed.balanceFingerprint, balanceFingerprint)
  } finally {
    db.close()
    rmSync(directory, { recursive: true, force: true })
  }
})
