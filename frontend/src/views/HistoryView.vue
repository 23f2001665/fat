<script setup>
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { toast } from 'vue3-toastify'
import EmptyState from '@/components/EmptyState.vue'
import Icon from '@/components/Icon.vue'
import PageHeader from '@/components/PageHeader.vue'
import SearchField from '@/components/SearchField.vue'
import StatCard from '@/components/StatCard.vue'
import { getWorkoutHistory } from '@/api/workouts'
import {
  formatDatePretty,
  formatDateShort,
  formatNumber,
  formatVolume,
  formatWeekday,
  pluralize,
} from '@/utils/format'

const router = useRouter()
const days = ref([])
const loading = ref(true)
const errorMessage = ref('')
const search = ref('')

const filteredDays = computed(() => {
  const needle = search.value.trim().toLowerCase()
  if (!needle) return days.value
  return days.value.filter(
    (day) =>
      day.date.includes(needle) ||
      day.exercise_names.some((name) => name.toLowerCase().includes(needle)),
  )
})

const totalSets = computed(() => days.value.reduce((sum, day) => sum + day.set_count, 0))
const totalVolume = computed(() => days.value.reduce((sum, day) => sum + day.total_volume, 0))
const totalExercises = computed(() => {
  const names = new Set()
  for (const day of days.value) {
    for (const name of day.exercise_names) names.add(name)
  }
  return names.size
})
const lastSession = computed(() => (days.value.length ? days.value[0].date : null))

const groups = computed(() => {
  const mapped = new Map()
  for (const day of filteredDays.value) {
    const key = day.date.slice(0, 7)
    if (!mapped.has(key)) mapped.set(key, [])
    mapped.get(key).push(day)
  }
  return Array.from(mapped.entries()).map(([key, rows]) => ({
    key,
    label: new Intl.DateTimeFormat('en-GB', {
      month: 'long',
      year: 'numeric',
    }).format(new Date(Number(key.slice(0, 4)), Number(key.slice(5, 7)) - 1, 1)),
    rows,
    volume: rows.reduce((sum, row) => sum + row.total_volume, 0),
  }))
})

async function loadHistory() {
  loading.value = true
  errorMessage.value = ''
  try {
    days.value = await getWorkoutHistory()
  } catch (error) {
    errorMessage.value = error.userMessage || 'Could not load your workout history.'
  } finally {
    loading.value = false
  }
}

function openDay(day) {
  router.push({ name: 'workout', query: { date: day.date } })
}

onMounted(loadHistory)
</script>

<template>
  <div class="page">
    <PageHeader
      eyebrow="Training archive"
      title="History"
      description="Every workout date in one place. Open any day to revisit, edit, or add sets."
    >
      <template #actions>
        <button class="btn btn-secondary" type="button" :disabled="loading" @click="loadHistory">
          <Icon name="refresh" :size="16" />
          Refresh
        </button>
        <RouterLink class="btn btn-primary" to="/workout">
          <Icon name="plus" :size="16" />
          Log a workout
        </RouterLink>
      </template>
    </PageHeader>

    <div v-if="errorMessage" class="notice notice-danger">
      <Icon name="alert" :size="18" class="notice-icon" />
      <span>{{ errorMessage }}</span>
      <button class="btn btn-sm btn-secondary retry-button" type="button" @click="loadHistory">
        Retry
      </button>
    </div>

    <div v-if="loading" class="summary-grid">
      <div v-for="index in 4" :key="index" class="card skeleton summary-skeleton" />
    </div>

    <div v-else-if="days.length" class="summary-grid">
      <StatCard
        label="Sessions"
        :value="days.length"
        icon="calendar"
        hint="Dates with recorded work"
        compact
      />
      <StatCard
        label="Total sets"
        :value="totalSets"
        icon="dumbbell"
        tone="info"
        hint="Across all sessions"
        compact
      />
      <StatCard
        label="Volume"
        :value="formatNumber(totalVolume, 1)"
        unit="kg"
        icon="chartBar"
        tone="violet"
        hint="All recorded volume"
        compact
      />
      <StatCard
        label="Exercises"
        :value="totalExercises"
        icon="library"
        tone="warning"
        hint="Distinct movements used"
        compact
      />
    </div>

    <section v-if="days.length && !loading" class="history-panel card">
      <header class="history-toolbar">
        <div>
          <h2>Workout days</h2>
          <p>{{ pluralize(filteredDays.length, 'session') }} shown</p>
        </div>
        <SearchField
          v-model="search"
          placeholder="Search date or exercise…"
          label="Search workout history"
        />
      </header>

      <div v-if="filteredDays.length" class="month-list">
        <div v-for="group in groups" :key="group.key" class="month-group">
          <div class="month-heading">
            <h3>{{ group.label }}</h3>
            <span>{{ pluralize(group.rows.length, 'session') }} · {{ formatVolume(group.volume) }}</span>
          </div>

          <div class="day-list">
            <button
              v-for="day in group.rows"
              :key="day.date"
              class="day-row"
              type="button"
              @click="openDay(day)"
            >
              <div class="date-block">
                <span class="date-weekday">{{ formatWeekday(day.date) }}</span>
                <strong>{{ formatDateShort(day.date).split(' ')[0] }}</strong>
                <span class="date-month">{{ formatDateShort(day.date).split(' ')[1] }}</span>
              </div>

              <div class="day-main">
                <div class="day-title">
                  <h3>{{ formatDatePretty(day.date) }}</h3>
                  <span class="badge badge-muted">{{ pluralize(day.set_count, 'set') }}</span>
                </div>
                <div class="exercise-tags">
                  <span
                    v-for="name in day.exercise_names.slice(0, 4)"
                    :key="name"
                    class="chip"
                  >
                    {{ name }}
                  </span>
                  <span v-if="day.exercise_names.length > 4" class="chip more-chip">
                    +{{ day.exercise_names.length - 4 }} more
                  </span>
                </div>
              </div>

              <div class="volume-block">
                <span>Volume</span>
                <strong>{{ formatVolume(day.total_volume) }}</strong>
              </div>

              <span class="open-arrow">
                <Icon name="chevronRight" :size="18" />
              </span>
            </button>
          </div>
        </div>
      </div>

      <EmptyState
        v-else
        icon="search"
        title="No matching sessions"
        message="Try a different exercise name or date, or clear the search."
      >
        <button class="btn btn-secondary" type="button" @click="search = ''">Clear search</button>
      </EmptyState>
    </section>

    <EmptyState
      v-else-if="!loading && !days.length"
      icon="history"
      title="Your history starts with one set"
      message="Once you log a workout, each training day will appear here with its sets, exercises, and volume."
    >
      <RouterLink class="btn btn-primary" to="/workout">
        <Icon name="plus" :size="16" />
        Log your first set
      </RouterLink>
    </EmptyState>
  </div>
</template>

<style scoped>
.retry-button {
  margin-left: auto;
}

.summary-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 13px;
}

.summary-skeleton {
  height: 105px;
}

.history-panel {
  overflow: hidden;
}

.history-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 18px;
  padding: 20px 22px;
  border-bottom: 1px solid var(--border);
}

.history-toolbar h2 {
  font-size: 1.05rem;
}

.history-toolbar p {
  margin-top: 4px;
  color: var(--muted);
  font-size: 0.74rem;
}

.month-list {
  display: flex;
  flex-direction: column;
}

.month-group {
  padding: 19px 22px 7px;
  border-bottom: 1px solid var(--border);
}

.month-group:last-child {
  border-bottom: 0;
}

.month-heading {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 10px;
}

.month-heading h3 {
  color: var(--text-soft);
  font-size: 0.78rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.month-heading span {
  color: var(--muted-2);
  font-size: 0.68rem;
}

.day-list {
  display: flex;
  flex-direction: column;
  gap: 7px;
}

.day-row {
  display: grid;
  width: 100%;
  grid-template-columns: 56px minmax(0, 1fr) auto 22px;
  align-items: center;
  gap: 15px;
  padding: 11px 10px;
  border: 1px solid transparent;
  border-radius: 13px;
  background: transparent;
  color: inherit;
  text-align: left;
  transition: all 150ms ease;
}

.day-row:hover {
  border-color: var(--border);
  background: var(--surface-2);
}

.date-block {
  display: flex;
  width: 54px;
  height: 58px;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--surface-2);
  line-height: 1;
}

.date-weekday {
  color: var(--accent);
  font-size: 0.56rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.date-block strong {
  margin: 4px 0 2px;
  font-family: var(--font-mono);
  font-size: 1.13rem;
  font-weight: 600;
}

.date-month {
  color: var(--muted-2);
  font-size: 0.55rem;
  font-weight: 600;
  text-transform: uppercase;
}

.day-main {
  min-width: 0;
}

.day-title {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 7px;
}

.day-title h3 {
  font-size: 0.87rem;
  font-weight: 600;
}

.exercise-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  margin-top: 6px;
}

.chip {
  min-height: 22px;
  padding: 2px 8px;
  color: var(--muted);
  font-size: 0.64rem;
}

.more-chip {
  border-color: transparent;
  background: transparent;
  color: var(--muted-2);
}

.volume-block {
  display: flex;
  min-width: 112px;
  flex-direction: column;
  align-items: flex-end;
  gap: 2px;
}

.volume-block span {
  color: var(--muted-2);
  font-size: 0.58rem;
  font-weight: 650;
  letter-spacing: 0.07em;
  text-transform: uppercase;
}

.volume-block strong {
  color: var(--text-soft);
  font-family: var(--font-mono);
  font-size: 0.8rem;
  font-weight: 550;
  white-space: nowrap;
}

.open-arrow {
  color: var(--muted-2);
  transition: transform 150ms ease;
}

.day-row:hover .open-arrow {
  color: var(--accent);
  transform: translateX(2px);
}

@media (max-width: 1000px) {
  .summary-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 700px) {
  .summary-grid {
    gap: 9px;
  }

  .history-toolbar {
    flex-direction: column;
    align-items: stretch;
    padding: 17px;
  }

  .month-group {
    padding: 16px 14px 4px;
  }

  .month-heading {
    align-items: flex-start;
    flex-direction: column;
    gap: 3px;
  }

  .day-row {
    grid-template-columns: 46px minmax(0, 1fr) 16px;
    gap: 10px;
    padding: 9px 5px;
  }

  .date-block {
    width: 44px;
    height: 51px;
  }

  .date-block strong {
    font-size: 1rem;
  }

  .volume-block {
    display: none;
  }

  .exercise-tags {
    max-height: 48px;
    overflow: hidden;
  }

  .open-arrow {
    justify-self: end;
  }
}
</style>
