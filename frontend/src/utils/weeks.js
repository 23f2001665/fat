import { parseISODate, shiftDateISO, todayISO } from './format'

const DAY_LETTERS = ['M', 'T', 'W', 'T', 'F', 'S', 'S']

/** Monday of the ISO week containing `iso` (YYYY-MM-DD). */
export function weekStartISO(iso) {
  const date = parseISODate(iso)
  if (!date) return iso
  const offset = (date.getDay() + 6) % 7
  return shiftDateISO(iso, -offset)
}

/**
 * Summarise training dates against a weekly session goal.
 * Only dates that actually have recorded sets count; nothing is inferred.
 */
export function summariseWeek(trainedDates, goal, today = todayISO()) {
  const trained = new Set(trainedDates)
  const start = weekStartISO(today)

  const days = DAY_LETTERS.map((letter, index) => {
    const iso = shiftDateISO(start, index)
    return {
      iso,
      letter,
      trained: trained.has(iso),
      isToday: iso === today,
      isFuture: iso > today,
    }
  })

  const sessions = days.filter((day) => day.trained).length
  const remaining = Math.max(0, goal - sessions)
  // Days from today to Sunday that could still hold a new session.
  const daysLeft = days.filter((day) => day.iso >= today && !day.trained).length
  const met = sessions >= goal

  const perWeek = new Map()
  for (const iso of trained) {
    const key = weekStartISO(iso)
    perWeek.set(key, (perWeek.get(key) || 0) + 1)
  }

  // Consecutive completed weeks that met the goal, counting back from last week.
  let previousStreak = 0
  let cursor = shiftDateISO(start, -7)
  while ((perWeek.get(cursor) || 0) >= goal) {
    previousStreak += 1
    cursor = shiftDateISO(cursor, -7)
  }

  return {
    days,
    sessions,
    goal,
    remaining,
    daysLeft,
    met,
    previousStreak,
    streak: previousStreak + (met ? 1 : 0),
  }
}
