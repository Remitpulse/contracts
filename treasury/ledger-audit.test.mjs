import assert from 'node:assert/strict'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { DatabaseSync } from 'node:sqlite'
import { test } from 'node:test'
import { Account, Asset, Keypair, Networks, Operation, TransactionBuilder } from '@stellar/stellar-sdk'
import { auditLedger } from './ledger-audit.mjs'

test('audits exact liabilities and can read snapshots from before the failed-deposit migration', () => {
  const directory = mkdtempSync(join(tmpdir(), 'rp-ledger-audit-'))
  const file = join(directory, 'ledger.sqlite')
  const db = new DatabaseSync(file)
  try {
    db.exec(`
      CREATE TABLE player (id TEXT PRIMARY KEY, testnet INTEGER, wallet TEXT);
      INSERT INTO player VALUES ('one', 9007199254740992, NULL), ('two', 1, NULL);
      CREATE TABLE entry (player TEXT, kind TEXT, amount INTEGER, detail TEXT);
      INSERT INTO entry VALUES
        ('one', 'Testnet deposit', 9007199254740995, 'credit'),
        ('one', 'Withdrawal reserved', -3, 'pending'),
        ('two', 'Testnet deposit', 8, 'credit-two'),
        ('two', 'Withdrawal reserved', -7, 'confirmed'),
        ('two', 'Demo grant', 10000000000, 'demo'),
        ('two', 'Testnet withdrawal confirmed', 0, 'confirmed');
      CREATE TABLE withdrawal (hash TEXT, player TEXT, pending INTEGER, amount TEXT, xdr TEXT);
      INSERT INTO withdrawal VALUES ('pending', 'one', 1, '3', 'signed-withdrawal'), ('confirmed', 'two', 0, '7', 'confirmed-xdr');
      CREATE TABLE deposit (hash TEXT, player TEXT, credited INTEGER, amount TEXT, wallet TEXT);
      CREATE TABLE signed_deposit (hash TEXT, xdr TEXT);
      INSERT INTO deposit VALUES ('failed', 'one', 0, '11', 'wallet'), ('unsigned', 'one', 0, '13', 'wallet'), ('credit', 'one', 1, '9007199254740995', 'wallet'), ('credit-two', 'two', 1, '8', 'wallet');
      INSERT INTO signed_deposit VALUES ('failed', 'signed-deposit'), ('credit', 'credited-one'), ('credit-two', 'credited-two');
    `)
    db.exec("UPDATE entry SET amount = 1 WHERE kind = 'Testnet withdrawal confirmed'")
    assert.equal(auditLedger(file).withdrawalOutcomeMismatches, 1)
    db.exec("UPDATE entry SET amount = 0 WHERE kind = 'Testnet withdrawal confirmed'")
    db.exec("UPDATE entry SET detail = 'orphan-reserve' WHERE kind = 'Withdrawal reserved' AND detail = 'confirmed'")
    assert.equal(auditLedger(file).reservationMismatches, 2)
    assert.equal(auditLedger(file).pendingReservationMismatches, 0)
    db.exec("UPDATE entry SET detail = 'confirmed' WHERE detail = 'orphan-reserve'")
    const { balanceFingerprint, pendingFingerprint } = auditLedger(file)
    assert.match(balanceFingerprint, /^[a-f0-9]{64}$/)
    const expected = {
      pendingFingerprint, balanceFingerprint, players: 2, balanceMismatches: 0, reservationMismatches: 0, pendingReservationMismatches: 0, withdrawalOutcomeMismatches: 0, pendingWithoutPlayer: 0, invalidPendingEnvelopes: 2, depositCreditMismatches: 0, failedDepositMismatches: 0, availableStroops: '9007199254740993', reservedStroops: '3',
      liabilitiesStroops: '9007199254740996', pendingDeposits: 1, pendingWithdrawals: 1,
    }
    assert.deepEqual(auditLedger(file), expected)
    db.exec("UPDATE deposit SET player = 'missing' WHERE hash = 'failed'")
    assert.equal(auditLedger(file).pendingWithoutPlayer, 1)
    db.exec("UPDATE deposit SET player = 'two' WHERE hash = 'failed'")
    assert.notEqual(auditLedger(file).pendingFingerprint, pendingFingerprint)
    db.exec("UPDATE deposit SET player = 'one' WHERE hash = 'failed'")
    assert.equal(auditLedger(file).pendingFingerprint, pendingFingerprint)
    db.exec("UPDATE signed_deposit SET xdr = 'damaged' WHERE hash = 'failed'")
    assert.notEqual(auditLedger(file).pendingFingerprint, pendingFingerprint)
    db.exec("UPDATE signed_deposit SET xdr = 'signed-deposit' WHERE hash = 'failed'")
    assert.equal(auditLedger(file).pendingFingerprint, pendingFingerprint)
    db.exec("CREATE TABLE failed_deposit (hash TEXT); INSERT INTO failed_deposit VALUES ('failed')")
    db.exec("INSERT INTO failed_deposit VALUES ('credit'), ('unsigned'), ('unknown')")
    assert.equal(auditLedger(file).failedDepositMismatches, 3)
    db.exec("DELETE FROM failed_deposit WHERE hash <> 'failed'")
    const migrated = auditLedger(file)
    assert.deepEqual(migrated, { ...expected, pendingFingerprint: migrated.pendingFingerprint, invalidPendingEnvelopes: 1, pendingDeposits: 0 })
    assert.notEqual(migrated.pendingFingerprint, pendingFingerprint)
    const signer = Keypair.random()
    const destination = Keypair.random().publicKey()
    db.prepare("UPDATE player SET wallet = ? WHERE id = 'one'").run(destination)
    const tx = new TransactionBuilder(new Account(signer.publicKey(), '0'), {
      fee: '100', networkPassphrase: Networks.TESTNET,
    }).setTimebounds(0, 1800000000)
      .addOperation(Operation.payment({ destination, asset: Asset.native(), amount: '0.0000003' }))
      .build()
    tx.sign(signer)
    const recordedHash = Buffer.from(tx.hash()).toString('hex')
    db.prepare("UPDATE withdrawal SET hash = ?, xdr = ? WHERE pending = 1").run(recordedHash, tx.toXdr())
    db.prepare("UPDATE entry SET detail = ? WHERE kind = 'Withdrawal reserved' AND detail = 'pending'").run(recordedHash)
    assert.equal(auditLedger(file).invalidPendingEnvelopes, 0)
    db.exec("UPDATE player SET wallet = 'wrong-wallet' WHERE id = 'one'")
    assert.equal(auditLedger(file).invalidPendingEnvelopes, 1)
    db.prepare("UPDATE player SET wallet = ? WHERE id = 'one'").run(destination)
    db.exec("UPDATE withdrawal SET amount = '4' WHERE pending = 1")
    assert.equal(auditLedger(file).invalidPendingEnvelopes, 1)
    db.exec("UPDATE withdrawal SET amount = '3' WHERE pending = 1")
    db.exec("UPDATE withdrawal SET hash = 'replacement' WHERE pending = 1")
    assert.notEqual(auditLedger(file).pendingFingerprint, migrated.pendingFingerprint)
    assert.equal(auditLedger(file).reservationMismatches, 2)
    db.exec("UPDATE player SET testnet = testnet + CASE id WHEN 'one' THEN 1 ELSE -1 END")
    const changed = auditLedger(file)
    assert.equal(changed.liabilitiesStroops, expected.liabilitiesStroops)
    assert.notEqual(changed.balanceFingerprint, balanceFingerprint)
    assert.equal(changed.balanceMismatches, 2)
    db.exec("INSERT INTO entry VALUES ('orphan', 'Testnet deposit', 1, 'orphan-credit')")
    assert.equal(auditLedger(file).balanceMismatches, 3)
    assert.equal(auditLedger(file).depositCreditMismatches, 1)
    db.exec("UPDATE deposit SET amount = '2' WHERE hash = 'credit-two'")
    assert.equal(auditLedger(file).depositCreditMismatches, 2)
  } finally {
    db.close()
    rmSync(directory, { recursive: true, force: true })
  }
})
