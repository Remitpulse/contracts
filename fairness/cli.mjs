import { readFile } from 'node:fs/promises'
import { verifyDiceReceipt } from './dice.mjs'

try {
  const [file, expectedCommitment, ...extra] = process.argv.slice(2)
  if (!file || extra.length) {
    throw new Error('Usage: npm run fairness:verify -- <dice-receipt.json> [recorded-commitment]')
  }
  const receipt = JSON.parse(await readFile(file, 'utf8'))
  if (!verifyDiceReceipt(receipt)) {
    throw new Error('The Dice commitment, roll or payout did not match.')
  }
  if (expectedCommitment !== undefined && receipt.commitment !== expectedCommitment) {
    throw new Error('The receipt does not match the separately recorded commitment.')
  }
  console.log('Verified Dice commitment, roll and payout.')
} catch (error) {
  console.error(error.message)
  process.exitCode = 1
}
