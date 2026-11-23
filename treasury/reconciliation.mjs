const unsigned = /^(?:0|[1-9]\d{0,29})$/

function stroops(value) {
  if (typeof value !== 'string' || !unsigned.test(value)) {
    throw new Error('Expected a non-negative integer stroop amount.')
  }
  return BigInt(value)
}

export function verifyReconciliation(summary) {
  if (summary?.network !== 'stellar-testnet') throw new Error('Expected a Testnet reconciliation summary.')
  const available = stroops(summary.availableStroops)
  const reserved = stroops(summary.reservedStroops)
  const liabilities = stroops(summary.liabilitiesStroops)
  if (available + reserved !== liabilities) throw new Error('Recorded liabilities do not match available and reserved funds.')
  if (summary.treasuryStroops === null) {
    if (summary.grossDifferenceStroops !== null) throw new Error('A treasury difference requires a treasury balance.')
  } else if ((stroops(summary.treasuryStroops) - liabilities).toString() !== summary.grossDifferenceStroops) {
    throw new Error('Recorded treasury difference does not match treasury balance and liabilities.')
  }
  return { network: summary.network, liabilitiesStroops: liabilities.toString(), treasuryCompared: summary.treasuryStroops !== null }
}

export function compareLedgerSummary(summary, ledger) {
  const fields = ['availableStroops', 'reservedStroops', 'liabilitiesStroops']
  const differences = fields.filter(field => summary[field] !== ledger[field])
  if (summary.ledgerChecks) {
    const checks = summary.ledgerChecks
    if (checks.mismatchedPlayers + checks.missingPlayers !== ledger.balanceMismatches) {
      differences.push('ledgerChecks.balanceMismatches')
    }
    for (const field of ['reservationMismatches', 'depositCreditMismatches']) {
      if (checks[field] !== undefined && checks[field] !== ledger[field]) {
        differences.push('ledgerChecks.' + field)
      }
    }
  }
  if (differences.length) {
    throw new Error('The ledger snapshot differs from the summary in: ' + differences.join(', '))
  }
}
