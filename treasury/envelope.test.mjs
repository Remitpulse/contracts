import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test } from 'node:test'
import { Account, Keypair, Networks, Operation, TransactionBuilder } from '@stellar/stellar-sdk'
import { inspectEnvelope } from './envelope.mjs'

test('inspects the signed envelope expiry only when its Testnet hash matches the ledger', () => {
  const signer = Keypair.random()
  const tx = new TransactionBuilder(new Account(signer.publicKey(), '0'), {
    fee: '100', networkPassphrase: Networks.TESTNET,
  }).setTimebounds(0, 1800000000)
    .addOperation(Operation.bumpSequence({ bumpTo: '5' })).build()
  tx.sign(signer)
  const hash = Buffer.from(tx.hash()).toString('hex')
  assert.deepEqual(inspectEnvelope(tx.toXdr(), hash), {
    hash, expiresAt: '2027-01-15T08:00:00.000Z', signatures: 1,
  })
  const mainnetHash = Buffer.from(TransactionBuilder.fromXDR(tx.toXdr(), Networks.PUBLIC).hash()).toString('hex')
  assert.throws(() => inspectEnvelope(tx.toXdr(), mainnetHash), /does not match/)

  const directory = mkdtempSync(join(tmpdir(), 'rp-envelope-'))
  try {
    const file = join(directory, 'transfer.xdr')
    writeFileSync(file, tx.toXdr(), { mode: 0o600 })
    const cli = fileURLToPath(new URL('./cli.mjs', import.meta.url))
    const output = execFileSync(process.execPath, [cli, 'envelope', file, hash], { encoding: 'utf8' })
    assert.deepEqual(JSON.parse(output), {
      network: 'Stellar Testnet', hash, expiresAt: '2027-01-15T08:00:00.000Z', signatures: 1,
    })
    assert.equal(output.includes(tx.toXdr()), false)
  } finally {
    rmSync(directory, { recursive: true, force: true })
  }
})
