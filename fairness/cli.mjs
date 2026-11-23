import { readFile } from 'node:fs/promises'
import { verifyDiceReceipt } from './dice.mjs'
import { verifySportsPayout } from './sports.mjs'

try {
  const args = process.argv.slice(2)
  const sports = args[0] === '--sports'
  const [file, expectedCommitment, ...extra] = sports ? args.slice(1) : args
  if (!file || extra.length || (sports && expectedCommitment !== undefined)) {
    throw new Error('Use <dice-receipt.json> [recorded-commitment] or --sports <sports-ticket.json>.')
  }
  const receipt = JSON.parse(await readFile(file, 'utf8'))
  if (sports) {
    if (!verifySportsPayout(receipt)) throw new Error('The simulated sports payout did not match.')
    console.log('Verified simulated sports payout arithmetic.')
  } else {
    verifyDice(receipt, expectedCommitment)
  }
} catch (error) {
  console.error(error.message)
  process.exitCode = 1
}

function verifyDice(receipt, expectedCommitment) {
  if (!verifyDiceReceipt(receipt)) {
    throw new Error('The Dice commitment, roll or payout did not match.')
  }
  if (expectedCommitment !== undefined && receipt.commitment !== expectedCommitment) {
    throw new Error('The receipt does not match the separately recorded commitment.')
  }
  console.log('Verified Dice commitment, roll and payout.')
}
