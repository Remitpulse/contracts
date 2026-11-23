import { auditLedger } from './ledger-audit.mjs'

try {
  const [file, restored, ...extra] = process.argv.slice(2)
  if (!file || extra.length) throw new Error('Usage: npm run treasury:audit -- <ledger.sqlite> [restored.sqlite]')
  const source = auditLedger(file)
  if (restored && JSON.stringify(source) !== JSON.stringify(auditLedger(restored))) {
    throw new Error('The restored ledger does not match the source audit.')
  }
  console.log(JSON.stringify(restored ? { ...source, restoredMatches: true } : source, null, 2))
} catch (error) {
  console.error(error.message)
  process.exitCode = 1
}
