import { createHash } from 'node:crypto'
import { DatabaseSync } from 'node:sqlite'

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
    const withdrawals = db.prepare('SELECT amount FROM withdrawal WHERE pending = 1').all()
    const reserved = withdrawals.reduce((sum, row) => sum + BigInt(row.amount), 0n)
    const supportsFailedDeposits = db.prepare(
      "SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = 'failed_deposit'",
    ).get()
    const pendingDeposits = db.prepare(
      `SELECT COUNT(*) AS count FROM deposit JOIN signed_deposit USING (hash) WHERE credited = 0
       ${supportsFailedDeposits ? 'AND hash NOT IN (SELECT hash FROM failed_deposit)' : ''}`,
    ).get()
    const fingerprint = createHash('sha256')
    const balances = db.prepare('SELECT id, CAST(testnet AS TEXT) AS stroops FROM player ORDER BY id')
    for (const { id, stroops } of balances.iterate()) {
      fingerprint.update(JSON.stringify([id, stroops]) + '\n')
    }
    return {
      balanceFingerprint: fingerprint.digest('hex'),
      players: players.count,
      availableStroops: players.available,
      reservedStroops: reserved.toString(),
      liabilitiesStroops: (BigInt(players.available) + reserved).toString(),
      pendingDeposits: pendingDeposits.count,
      pendingWithdrawals: withdrawals.length,
    }
  } finally {
    db.close()
  }
}
