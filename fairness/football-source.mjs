import { verifySportsPayout } from './sports.mjs'

export async function verifyFootballSource(ticket, token, fetcher = fetch) {
  const id = /^football-data:([1-9][0-9]{0,14})$/.exec(ticket?.event ?? '')?.[1]
  if (!id || !verifySportsPayout(ticket)) throw new Error('Expected a settled provider football ticket.')
  if (!token) throw new Error('Set CASINO_FOOTBALL_DATA_TOKEN before checking the provider.')
  const response = await fetcher('https://api.football-data.org/v4/matches/' + id, {
    headers: { 'X-Auth-Token': token },
    signal: AbortSignal.timeout(6000),
    redirect: 'error',
  })
  if (!response.ok) throw new Error('Football provider returned HTTP ' + response.status + '.')
  const match = await response.json()
  if (match?.id !== Number(id)) throw new Error('The provider did not return the requested match.')
  if (ticket.state === 'Void') return match.status === ticket.score
  if (match.status !== 'FINISHED') return false
  const score = match.score?.duration === 'REGULAR' ? match.score.fullTime : match.score?.regularTime
  if (!Number.isSafeInteger(score?.home) || score.home < 0 ||
      !Number.isSafeInteger(score?.away) || score.away < 0) return false
  return ticket.score === `${score.home}–${score.away}`
}
