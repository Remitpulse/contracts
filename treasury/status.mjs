import { hash } from './native.mjs'

export async function lookupTransfer(horizon, transactionHash) {
  const id = hash(transactionHash)
  try {
    const transaction = await horizon.transactions().transaction(id).call()
    if (transaction?.hash !== id || typeof transaction.successful !== 'boolean' ||
        !Number.isSafeInteger(transaction.ledger_attr) || transaction.ledger_attr <= 0) {
      throw new Error('Horizon returned an incomplete or mismatched transfer receipt.')
    }
    return {
      hash: id,
      status: transaction.successful ? 'confirmed' : 'failed',
      ledger: transaction.ledger_attr,
    }
  } catch (error) {
    if (error?.response?.status === 404) return { hash: id, status: 'not-found', ledger: null }
    throw error
  }
}
