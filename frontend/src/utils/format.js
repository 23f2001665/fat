export function todayISO() {
  const now = new Date()
  const year = now.getFullYear()
  const month = String(now.getMonth() + 1).padStart(2, '0')
  const day = String(now.getDate()).padStart(2, '0')
  return `${year}-${month}-${day}`
}

export function parseISODate(value) {
  if (!value) return null
  const [year, month, day] = value.split('-').map(Number)
  if (!year || !month || !day) return null
  return new Date(year, month - 1, day)
}

export function shiftDateISO(value, days) {
  const date = parseISODate(value)
  if (!date) return value
  date.setDate(date.getDate() + days)
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(
    date.getDate(),
  ).padStart(2, '0')}`
}

export function isFutureISO(value) {
  return Boolean(value) && value > todayISO()
}

export function formatNumber(value, maximumFractionDigits = 1) {
  if (value === null || value === undefined || Number.isNaN(Number(value))) return '—'
  return new Intl.NumberFormat('en-US', {
    maximumFractionDigits,
    minimumFractionDigits: 0,
  }).format(Number(value))
}

export function formatWeight(value, unit = 'kg') {
  if (value === null || value === undefined) return '—'
  return `${formatNumber(value, 2)} ${unit}`
}

export function formatVolume(value) {
  if (value === null || value === undefined) return '—'
  return `${formatNumber(value, 1)} kg`
}

export function formatCompact(value) {
  if (value === null || value === undefined) return '—'
  const number = Number(value)
  if (Math.abs(number) >= 1_000_000) {
    return `${formatNumber(number / 1_000_000, 1)}M`
  }
  if (Math.abs(number) >= 10_000) {
    return `${formatNumber(number / 1_000, 1)}k`
  }
  return formatNumber(number, 1)
}

export function formatDatePretty(value) {
  const date = parseISODate(value)
  if (!date) return '—'
  return new Intl.DateTimeFormat('en-GB', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  }).format(date)
}

export function formatDateLong(value) {
  const date = parseISODate(value)
  if (!date) return '—'
  return new Intl.DateTimeFormat('en-GB', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  }).format(date)
}

export function formatDateShort(value) {
  const date = parseISODate(value)
  if (!date) return '—'
  return new Intl.DateTimeFormat('en-GB', {
    day: 'numeric',
    month: 'short',
  }).format(date)
}

export function formatWeekday(value) {
  const date = parseISODate(value)
  if (!date) return ''
  return new Intl.DateTimeFormat('en-GB', { weekday: 'short' }).format(date)
}

export function formatWeekLabel(value) {
  return `Wk of ${formatDateShort(value)}`
}

export function formatRest(seconds) {
  if (seconds === null || seconds === undefined || seconds === '') return '—'
  const total = Math.max(0, Number(seconds))
  if (total < 60) return `${formatNumber(total, 0)}s`
  const minutes = Math.floor(total / 60)
  const remainder = total % 60
  if (minutes < 60) {
    return remainder ? `${minutes}m ${remainder}s` : `${minutes}m`
  }
  const hours = Math.floor(minutes / 60)
  const remainingMinutes = minutes % 60
  return remainingMinutes ? `${hours}h ${remainingMinutes}m` : `${hours}h`
}

export function formatDelta(value, unit = '') {
  if (value === null || value === undefined) return null
  const number = Number(value)
  const sign = number > 0 ? '+' : ''
  return `${sign}${formatNumber(number, 2)}${unit ? ` ${unit}` : ''}`
}

export function pluralize(count, singular, plural = `${singular}s`) {
  return `${count} ${Number(count) === 1 ? singular : plural}`
}

export function initials(name) {
  if (!name) return 'A'
  return name
    .trim()
    .split(/\s+/)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase() || '')
    .join('')
}

export function sortByName(rows) {
  return [...rows].sort((a, b) => {
    if (a.is_active !== b.is_active) return a.is_active ? -1 : 1
    return a.name.localeCompare(b.name, undefined, { sensitivity: 'base' })
  })
}

export function titleCase(value) {
  if (!value) return ''
  return value.replace(/\b\w/g, (character) => character.toUpperCase())
}
