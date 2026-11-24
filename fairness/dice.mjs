import { createHash, createHmac } from 'node:crypto'

export function verifyDiceReceipt(proof) {
  const seed = /^[a-f0-9]{64}$/
  if (proof?.version !== 1 || !seed.test(proof.serverSeed) || !seed.test(proof.clientSeed)
      || !Number.isInteger(proof.target) || proof.target < 2 || proof.target > 98
      || typeof proof.stake !== 'string' || !/^[1-9]\d{0,9}$/.test(proof.stake)
      || BigInt(proof.stake) > 1_000_000_000n) return false
  const key = Buffer.from(proof.serverSeed, 'hex')
  if (createHash('sha256').update(key).digest('hex') !== proof.commitment) return false
  for (let counter = 0; ; counter++) {
    const draw = createHmac('sha256', key)
      .update(`remitpulse:dice:v1:${proof.clientSeed}:${counter}`)
      .digest().readUInt32BE(0)
    if (draw >= 4_294_960_000) continue
    const roll = draw % 10_000
    const payout = roll < proof.target * 100 ? (BigInt(proof.stake) * 99n) / BigInt(proof.target) : 0n
    return proof.roll === roll && proof.payout === payout.toString()
  }
}
