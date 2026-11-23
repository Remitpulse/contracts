import { createHash } from 'node:crypto'
import { DatabaseSync } from 'node:sqlite'
import { matchesPaymentEnvelope } from './envelope.mjs'

export function auditLedger(file) {
  const db = new DatabaseSync(file, { readOnly: true })
  try {
    db.exec('BEGIN')
    const integrity = db.prepare('PRAGMA integrity_check').all()
    if (integrity.length !== 1 || integrity[0].integrity_check !== 'ok') {
      throw new Error('SQLite integrity check failed.')
    }
    const players = db.prepare(
      'SELECT COUNT(*) AS count, CAST(COALESCE(SUM(testnet), 0) AS TEXT) AS available FROM player',
    ).get()
    const withdrawals = db.prepare('SELECT hash, player, amount, xdr FROM withdrawal WHERE pending = 1 ORDER BY hash').all()
    const reserved = withdrawals.reduce((sum, row) => sum + BigInt(row.amount), 0n)
    const supportsFailedDeposits = db.prepare(
      "SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = 'failed_deposit'",
    ).get()
    const pendingDeposits = db.prepare(
      `SELECT hash, player, amount, signed_deposit.xdr FROM deposit JOIN signed_deposit USING (hash) WHERE credited = 0
       ${supportsFailedDeposits ? 'AND hash NOT IN (SELECT hash FROM failed_deposit)' : ''} ORDER BY hash`,
    ).all()
    const mismatch = db.prepare(
      `WITH movement AS (
        SELECT player, SUM(amount) AS net FROM entry
        WHERE kind IN ('Testnet deposit', 'Withdrawal reserved', 'Failed Testnet withdrawal released')
        GROUP BY player
      )
      SELECT (SELECT COUNT(*) FROM player p LEFT JOIN movement m ON m.player = p.id
        WHERE p.testnet <> COALESCE(m.net, 0))
      + (SELECT COUNT(*) FROM movement m LEFT JOIN player p ON p.id = m.player
        WHERE p.id IS NULL) AS count`,
    ).get()
    const reservations = db.prepare(
      `WITH reservation AS (
         SELECT detail, COUNT(*) AS entries, MIN(player) AS player, MIN(amount) AS amount
         FROM entry WHERE kind = 'Withdrawal reserved' GROUP BY detail
       )
       SELECT COUNT(*) AS count FROM withdrawal w
       LEFT JOIN reservation r ON r.detail = w.hash
       WHERE w.pending = 1 AND (COALESCE(r.entries, 0) <> 1 OR r.player <> w.player
         OR CAST(-r.amount AS TEXT) <> w.amount)`,
    ).get()
    const depositCredits = db.prepare(
      `WITH credit AS (
         SELECT detail, COUNT(*) AS entries, MIN(player) AS player, MIN(amount) AS amount
         FROM entry WHERE kind = 'Testnet deposit' GROUP BY detail
       )
       SELECT (SELECT COUNT(*) FROM deposit d LEFT JOIN credit c ON c.detail = d.hash
         WHERE d.credited = 1 AND (COALESCE(c.entries, 0) <> 1 OR c.player <> d.player
           OR CAST(c.amount AS TEXT) <> d.amount))
       + (SELECT COUNT(*) FROM credit c LEFT JOIN deposit d ON d.hash = c.detail
         WHERE d.credited IS NULL OR d.credited <> 1) AS count`,
    ).get()
    const pendingWithoutPlayer = db.prepare(
      `SELECT (SELECT COUNT(*) FROM withdrawal w LEFT JOIN player p ON p.id = w.player
         WHERE w.pending = 1 AND p.id IS NULL)
       + (SELECT COUNT(*) FROM deposit d JOIN signed_deposit s USING (hash)
         LEFT JOIN player p ON p.id = d.player
         WHERE d.credited = 0 AND p.id IS NULL
           ${supportsFailedDeposits ? 'AND d.hash NOT IN (SELECT hash FROM failed_deposit)' : ''}) AS count`,
    ).get()
    const fingerprint = createHash('sha256')
    const balances = db.prepare('SELECT id, CAST(testnet AS TEXT) AS stroops FROM player ORDER BY id')
    for (const { id, stroops } of balances.iterate()) {
      fingerprint.update(JSON.stringify([id, stroops]) + '\n')
    }
    const pendingFingerprint = createHash('sha256')
    for (const { hash, player, amount, xdr } of pendingDeposits) {
      pendingFingerprint.update(JSON.stringify(['deposit', hash, player, amount, xdr]) + '\n')
    }
    for (const { hash, player, amount, xdr } of withdrawals) {
      pendingFingerprint.update(JSON.stringify(['withdrawal', hash, player, amount, xdr]) + '\n')
    }
    let invalidPendingEnvelopes = 0
    for (const { hash, xdr, amount } of [...pendingDeposits, ...withdrawals]) {
      try {
        if (!matchesPaymentEnvelope(xdr, hash, amount)) invalidPendingEnvelopes++
      } catch {
        invalidPendingEnvelopes++
      }
    }
    return {
      invalidPendingEnvelopes,
      pendingFingerprint: pendingFingerprint.digest('hex'),
      balanceFingerprint: fingerprint.digest('hex'),
      players: players.count,
      balanceMismatches: mismatch.count,
      reservationMismatches: reservations.count,
      pendingWithoutPlayer: pendingWithoutPlayer.count,
      depositCreditMismatches: depositCredits.count,
      availableStroops: players.available,
      reservedStroops: reserved.toString(),
      liabilitiesStroops: (BigInt(players.available) + reserved).toString(),
      pendingDeposits: pendingDeposits.length,
      pendingWithdrawals: withdrawals.length,
    }
  } finally {
    db.close()
  }
}
