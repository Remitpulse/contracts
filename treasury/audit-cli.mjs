import { auditLedger } from './ledger-audit.mjs'

try {
  const [file, ...extra] = process.argv.slice(2)
  if (!file || extra.length) throw new Error('Usage: npm run treasury:audit -- <ledger.sqlite>')
  console.log(JSON.stringify(auditLedger(file), null, 2))
} catch (error) {
  console.error(error.message)
  process.exitCode = 1
}
