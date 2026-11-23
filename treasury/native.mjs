import { StrKey } from '@stellar/stellar-sdk'

export const horizonUrl = 'https://horizon-testnet.stellar.org'

export function publicKey(raw) {
  if (typeof raw !== 'string' || !StrKey.isValidEd25519PublicKey(raw)) {
    throw new Error('Provide a Stellar G-address.')
  }
  return raw
}

export function hash(raw) {
  if (typeof raw !== 'string' || !/^[a-f0-9]{64}$/.test(raw)) {
    throw new Error('Provide a lowercase Stellar transaction hash.')
  }
  return raw
}

export function stroops(xlm) {
  if (typeof xlm !== 'string' || !/^\d{1,12}(\.\d{1,7})?$/.test(xlm)) {
    throw new Error('Expected an exact native XLM amount.')
  }
  const [whole, fraction = ''] = xlm.split('.')
  return (BigInt(whole) * 10_000_000n + BigInt(fraction.padEnd(7, '0'))).toString()
}

export function nativePayments(operations, treasury) {
  return operations.flatMap(operation => {
    if (operation.type !== 'payment' || operation.asset_type !== 'native' || operation.from === operation.to ||
        (operation.to !== treasury && operation.from !== treasury)) return []
    return [{
      direction: operation.to === treasury ? 'inbound' : 'outbound',
      from: operation.from,
      to: operation.to,
      amount: operation.amount,
      asset: 'XLM',
      stroops: stroops(operation.amount),
    }]
  })
}

export function receiptExpectation(direction, amount, counterparty) {
  if (direction !== 'inbound' && direction !== 'outbound') {
    throw new Error('Provide the expected direction as inbound or outbound.')
  }
  if (typeof amount !== 'string' || !/^[1-9]\d{0,18}$/.test(amount) || BigInt(amount) > 9_223_372_036_854_775_807n) {
    throw new Error('Provide an exact positive amount in integer stroops.')
  }
  return { direction, stroops: amount, counterparty: publicKey(counterparty) }
}

export function assertReceiptMatches(payment, expected) {
  const counterparty = payment.direction === 'inbound' ? payment.from : payment.to
  if (payment.direction !== expected.direction || payment.stroops !== expected.stroops ||
      counterparty !== expected.counterparty) {
    throw new Error('The treasury payment does not match the recorded direction, amount, and wallet.')
  }
}
