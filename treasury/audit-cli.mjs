import { auditLedger } from './ledger-audit.mjs'

try {
  const [file, restored, ...extra] = process.argv.slice(2)
  if (!file || extra.length) throw new Error('Usage: npm run treasury:audit -- <ledger.sqlite> [restored.sqlite]')
  const source = auditLedger(file)
  if (restored && JSON.stringify(source) !== JSON.stringify(auditLedger(restored))) {
    throw new Error('The restored ledger does not match the source audit.')
  }
  console.log(JSON.stringify(restored ? { ...source, restoredMatches: true } : source, null, 2))
  if (source.balanceMismatches || source.reservationMismatches || source.depositCreditMismatches || source.pendingWithoutPlayer || source.invalidPendingEnvelopes) {
    console.error('The Testnet ledger contains reconciliation discrepancies.')
    process.exitCode = 1
  }
} catch (error) {
  console.error(error.message)
  process.exitCode = 1
}
