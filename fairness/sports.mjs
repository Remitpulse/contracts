const maxStake = 1_000_000_000n

export function verifySportsPayout(ticket) {
  if (!ticket || !Number.isSafeInteger(ticket.price) || ticket.price < 100 ||
      typeof ticket.stake !== 'string' || !/^[1-9]\d{0,9}$/.test(ticket.stake) ||
      typeof ticket.payout !== 'string') return false

  const stake = BigInt(ticket.stake)
  if (stake > maxStake) return false

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
