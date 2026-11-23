const maxStake = 1_000_000_000n

export function verifySportsPayout(ticket) {
  if (!ticket || !Number.isSafeInteger(ticket.price) || ticket.price < 100 ||
      typeof ticket.stake !== 'string' || !/^[1-9]\d{0,9}$/.test(ticket.stake) ||
      typeof ticket.payout !== 'string') return false

  const stake = BigInt(ticket.stake)
  if (stake > maxStake) return false

  if (typeof ticket.event === 'string' && ticket.event.startsWith('football-data:') &&
      !matchesFootballResult(ticket)) return false

  switch (ticket.state) {
    case 'Won':
      return ((stake * BigInt(ticket.price)) / 100n).toString() === ticket.payout
    case 'Void':
      return ticket.stake === ticket.payout
    case 'Lost':
      return ticket.payout === '0'
    default:
      return false
  }
}

function matchesFootballResult(ticket) {
  if (!Number.isInteger(ticket.selection) || ticket.selection < 0 || ticket.selection > 2) return false
  if (ticket.state === 'Void') return ticket.score === 'CANCELLED' || ticket.score === 'AWARDED'

  const score = typeof ticket.score === 'string' && /^(0|[1-9]\d{0,14})–(0|[1-9]\d{0,14})$/.exec(ticket.score)
  if (!score) return false
  const home = Number(score[1])
  const away = Number(score[2])
  const winningSelection = home === away ? 1 : home > away ? 0 : 2
  return ticket.state === (ticket.selection === winningSelection ? 'Won' : 'Lost')
}
