import { reactive, watch } from 'vue'
import { authState } from './auth'

// Smart default: three sessions a week is a common, achievable starting point.
export const DEFAULT_WEEKLY_GOAL = 3
export const WEEKLY_GOAL_OPTIONS = [
  { value: 2, note: 'Light' },
  { value: 3, note: 'Recommended' },
  { value: 4, note: '' },
  { value: 5, note: '' },
  { value: 6, note: 'Advanced' },
]

// Per-viewer preferences. Browser storage can be unavailable (private mode,
// blocked site data), so every access is guarded and the UI works without it.
export const goalState = reactive({
  weeklyGoal: DEFAULT_WEEKLY_GOAL,
  goalChosen: false,
  checklistDismissed: false,
})

function storageKey(name) {
  return `fat:${authState.user?.id ?? 'anon'}:${name}`
}

function read(name, fallback) {
  try {
    const raw = window.localStorage.getItem(storageKey(name))
    return raw === null ? fallback : JSON.parse(raw)
  } catch {
    return fallback
  }
}

function write(name, value) {
  try {
    window.localStorage.setItem(storageKey(name), JSON.stringify(value))
  } catch {
    // Preference simply won't persist; in-memory state still applies.
  }
}

function load() {
  const stored = Number(read('weeklyGoal', 0))
  goalState.goalChosen = stored >= 1 && stored <= 7
  goalState.weeklyGoal = goalState.goalChosen ? stored : DEFAULT_WEEKLY_GOAL
  goalState.checklistDismissed = Boolean(read('checklistDismissed', false))
}

watch(() => authState.user?.id, load, { immediate: true })

export function setWeeklyGoal(value) {
  goalState.weeklyGoal = value
  goalState.goalChosen = true
  write('weeklyGoal', value)
}

export function dismissChecklist() {
  goalState.checklistDismissed = true
  write('checklistDismissed', true)
}
