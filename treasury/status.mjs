import { hash } from './native.mjs'

export async function lookupTransfer(horizon, transactionHash) {
  const id = hash(transactionHash)
  try {
    const transaction = await horizon.transactions().transaction(id).call()
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
