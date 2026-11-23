import { Horizon } from '@stellar/stellar-sdk'
import { hash, horizonUrl, nativePayments, publicKey, stroops } from './native.mjs'

const [command, input, expectedTreasury] = process.argv.slice(2)
const horizon = new Horizon.Server(horizonUrl)
horizon.httpClient.defaults.timeout = 10000
try {
  let result
  if (command === 'balance') {
    const account = await horizon.loadAccount(publicKey(input))
    const native = account.balances.find(balance => balance.asset_type === 'native')
    if (!native) throw new Error('Account has no native XLM entry.')
    result = { network: 'Stellar Testnet', account: account.id, asset: 'XLM', amount: native.balance, stroops: stroops(native.balance) }
  } else if (command === 'receipt') {
    const transaction = await horizon.transactions().transaction(hash(input)).call()
    const treasury = publicKey(expectedTreasury)
    const operations = await horizon.operations().forTransaction(transaction.hash).limit(100).call()
    const payments = nativePayments(operations.records, treasury)
    if (!transaction.successful || payments.length !== 1 || operations.records.length !== 1) {
      throw new Error('Expected a successful transaction with exactly one native XLM treasury payment.')
    }
    result = { network: 'Stellar Testnet', hash: transaction.hash, ledger: transaction.ledger_attr, payment: payments[0] }
  } else {
    throw new Error('Use balance <treasury G-address> or receipt <hash> <treasury G-address>.')
  }
  console.log(JSON.stringify(result, null, 2))
} catch (error) {
  console.error('Treasury check failed:', error.message)
  process.exitCode = 1
}
