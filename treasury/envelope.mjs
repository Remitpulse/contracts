import { Keypair, Networks, Transaction, TransactionBuilder } from '@stellar/stellar-sdk'
import { hash, stroops } from './native.mjs'

function readEnvelope(envelope, recordedHash) {
  const transaction = TransactionBuilder.fromXDR(envelope.trim(), Networks.TESTNET)
  if (!(transaction instanceof Transaction)) throw new Error('Expected an original Stellar transaction.')
  const transactionHash = Buffer.from(transaction.hash()).toString('hex')
  if (transactionHash !== hash(recordedHash)) throw new Error('Envelope does not match the recorded Testnet hash.')
  return { transaction, hash: transactionHash }
}

export function inspectEnvelope(envelope, recordedHash) {
  const { transaction, hash: transactionHash } = readEnvelope(envelope, recordedHash)
  const maxTime = Number(transaction.timeBounds?.maxTime)
  const expiresAt = Number.isSafeInteger(maxTime) && maxTime > 0 && maxTime <= 8_640_000_000_000
    ? new Date(maxTime * 1000).toISOString()
    : null
  return { hash: transactionHash, expiresAt, signatures: transaction.signatures.length }
}

export function matchesPaymentEnvelope(envelope, recordedHash, amount, direction, counterparty) {
  const { transaction } = readEnvelope(envelope, recordedHash)
  const [payment] = transaction.operations
  const signer = Keypair.fromPublicKey(transaction.source)
  const signedBySource = transaction.signatures.some(signature =>
    signer.verify(transaction.hash(), signature.signature),
  )
  return signedBySource && transaction.operations.length === 1 &&
    payment.type === 'payment' && payment.asset.isNative() && stroops(payment.amount) === amount &&
    (direction === 'deposit'
      ? (payment.source ?? transaction.source) === counterparty
      : payment.destination === counterparty)
}
