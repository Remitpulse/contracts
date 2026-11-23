import { readFile } from 'node:fs/promises'
import { verifyDiceReceipt } from './dice.mjs'

try {
  const [file] = process.argv.slice(2)
  if (!file || process.argv.length !== 3) {
    throw new Error('Usage: npm run fairness:verify -- <dice-receipt.json>')
  }
  const receipt = JSON.parse(await readFile(file, 'utf8'))
  if (!verifyDiceReceipt(receipt)) {
    throw new Error('The Dice commitment, roll or payout did not match.')
  }
  console.log('Verified Dice commitment, roll and payout.')
} catch (error) {
  console.error(error.message)
  process.exitCode = 1
}
