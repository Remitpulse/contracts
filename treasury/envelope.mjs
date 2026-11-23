import { Networks, Transaction, TransactionBuilder } from '@stellar/stellar-sdk'
import { hash } from './native.mjs'

export function inspectEnvelope(envelope, recordedHash) {
  const transaction = TransactionBuilder.fromXDR(envelope.trim(), Networks.TESTNET)
  if (!(transaction instanceof Transaction)) throw new Error('Expected an original Stellar transaction.')
  const transactionHash = Buffer.from(transaction.hash()).toString('hex')
  if (transactionHash !== hash(recordedHash)) throw new Error('Envelope does not match the recorded Testnet hash.')
  const maxTime = Number(transaction.timeBounds?.maxTime)
  const expiresAt = Number.isSafeInteger(maxTime) && maxTime > 0 && maxTime <= 8_640_000_000_000
    ? new Date(maxTime * 1000).toISOString()
    : null
  return { hash: transactionHash, expiresAt, signatures: transaction.signatures.length }
}
