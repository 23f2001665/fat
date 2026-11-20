<script setup>
import { computed, onMounted, ref, watch } from 'vue'
import { toast } from 'vue3-toastify'
import EmptyState from '@/components/EmptyState.vue'
import Icon from '@/components/Icon.vue'
import PageHeader from '@/components/PageHeader.vue'
import StatCard from '@/components/StatCard.vue'
import BarChart from '@/components/charts/BarChart.vue'
import DonutChart from '@/components/charts/DonutChart.vue'
import LineChart from '@/components/charts/LineChart.vue'
import { getAnalytics } from '@/api/analytics'
import { getExercises, getMuscleGroups } from '@/api/library'
import {
  formatCompact,
  formatDatePretty,
  formatNumber,
  formatWeekLabel,
  shiftDateISO,
  todayISO,
} from '@/utils/format'

const today = todayISO()
const periods = [
  { key: '7', label: '7D', title: 'Last 7 days' },
  { key: '30', label: '30D', title: 'Last 30 days' },
  { key: '90', label: '90D', title: 'Last 90 days' },
  { key: '6m', label: '6M', title: 'Last 6 months' },
  { key: '12m', label: '12M', title: 'Last 12 months' },
  { key: 'all', label: 'All', title: 'All time' },
  { key: 'custom', label: 'Custom', title: 'Custom range' },
]
const chartOptions = [
  { key: 'weight', label: 'Weight', icon: 'arrowUp' },
  { key: 'volume', label: 'Volume', icon: 'chartBar' },
  { key: 'weekly', label: 'Weekly', icon: 'calendar' },
  { key: 'muscles', label: 'Muscles', icon: 'target' },
  { key: 'frequency', label: 'Frequency', icon: 'dumbbell' },
  { key: 'rir', label: 'RIR', icon: 'sparkles' },
]

const period = ref('30')
const customStart = ref(shiftDateISO(today, -29))
const customEnd = ref(today)
const exerciseId = ref('')
const muscleGroupId = ref('')
const exercises = ref([])
const muscleGroups = ref([])
const report = ref(null)
const loading = ref(true)
const errorMessage = ref('')
const chartKey = ref('weight')

const exerciseStats = computed(() => report.value?.exercise || {})
const hasData = computed(() => Boolean(report.value?.has_data))
const hasFilters = computed(() => Boolean(exerciseId.value || muscleGroupId.value))
const rangeLabel = computed(() => {
  const range = report.value?.range
  if (!range) return 'Loading range'
  if (!range.start && !range.end) return 'All recorded dates'
  if (range.start && range.end) {
    return `${formatDatePretty(range.start)} – ${formatDatePretty(range.end)}`
  }
  return range.start ? `From ${formatDatePretty(range.start)}` : `Until ${formatDatePretty(range.end)}`
})
const weeklyItems = computed(() =>
  (report.value?.weekly_volume || []).map((point) => ({
    label: formatWeekLabel(point.date),
    value: point.value,
  })),
)
const frequencyItems = computed(() =>
  (report.value?.exercise_frequency || []).map((row) => ({
    label: row.label,
    value: row.value,
  })),
)
const rirPoints = computed(() => exerciseStats.value.rir_trend || [])

const chartTitle = computed(() => {
  return {
    weight: 'Weight progression',
    volume: 'Volume progression',
    weekly: 'Weekly training volume',
    muscles: 'Attributed volume by muscle group',
    frequency: 'Exercise frequency',
    rir: 'Average RIR trend',
  }[chartKey.value]
})
const chartDescription = computed(() => {
  return {
    weight:
      'The heaviest recorded set on each workout date. Dates without a set are not filled in.',
    volume:
      'Total volume for every recorded session, using weight × effective reps.',
    weekly:
      'Volume grouped by the Monday of each training week. Empty weeks are omitted.',
    muscles:
      'Primary muscle involvement receives 100% of set volume; secondary receives 50%.',
    frequency:
      'Number of recorded sets per exercise in the selected range.',
    rir:
      'Average reps in reserve per workout date. Sets without an RIR entry are ignored.',
  }[chartKey.value]
})

async function loadReferences() {
  try {
    const [exerciseRows, muscleRows] = await Promise.all([
      getExercises({ includeInactive: true }),
      getMuscleGroups({ includeInactive: true }),
    ])
    exercises.value = exerciseRows
    muscleGroups.value = muscleRows
  } catch (error) {
    toast.error(error.userMessage || 'Could not load analytics filters.')
  }
}

async function loadReport() {
  if (period.value === 'custom' && (!customStart.value || !customEnd.value)) {
    toast.error('Choose both custom dates.')
    return
  }
  loading.value = true
  errorMessage.value = ''
  try {
    report.value = await getAnalytics({
      period: period.value,
      start: period.value === 'custom' ? customStart.value : undefined,
      end: period.value === 'custom' ? customEnd.value : undefined,
      exerciseId: exerciseId.value || undefined,
      muscleGroupId: muscleGroupId.value || undefined,
    })
  } catch (error) {
    errorMessage.value = error.userMessage || 'Could not build the analytics report.'
  } finally {
    loading.value = false
  }
}

function choosePeriod(key) {
  if (period.value === key) return
  period.value = key
  if (key === 'custom') return
  loadReport()
}

function applyCustomRange() {
  if (!customStart.value || !customEnd.value) {
    toast.error('Choose both custom dates.')
    return
  }
  if (customStart.value > customEnd.value) {
    toast.error('The start date must be on or before the end date.')
    return
  }
  loadReport()
}

function clearFilters() {
  exerciseId.value = ''
  muscleGroupId.value = ''
}

watch([exerciseId, muscleGroupId], () => {
  if (period.value === 'custom' && (!customStart.value || !customEnd.value)) return
  loadReport()
})

onMounted(() => {
  loadReferences()
  loadReport()
})
</script>

<template>
  <div class="page">
    <PageHeader
      title="Analytics"
      description="Trends built only from the sets you logged. Nothing is estimated."
    >
      <template #actions>
        <button class="btn btn-secondary" type="button" :disabled="loading" @click="loadReport">
          <Icon name="refresh" :size="16" />
          Refresh report
        </button>
      </template>
    </PageHeader>

    <section class="filter-panel card">
      <div class="filter-main">
        <div class="period-group">
          <span class="filter-label">Reporting period</span>
          <div class="segmented">
            <button
              v-for="item in periods"
              :key="item.key"
              class="segment"
              :class="{ active: period === item.key }"
              type="button"
              :title="item.title"
              @click="choosePeriod(item.key)"
            >
              {{ item.label }}
            </button>
          </div>
        </div>

        <div class="select-filters">
          <label class="field compact-field">
            <span class="field-label">Exercise</span>
            <select v-model="exerciseId" class="select">
              <option value="">All exercises</option>
              <option v-for="exercise in exercises" :key="exercise.id" :value="String(exercise.id)">
                {{ exercise.name }}{{ exercise.is_active ? '' : ' · inactive' }}
              </option>
            </select>
          </label>

          <label class="field compact-field">
            <span class="field-label">Muscle group</span>
            <select v-model="muscleGroupId" class="select">
              <option value="">All muscle groups</option>
              <option v-for="muscle in muscleGroups" :key="muscle.id" :value="String(muscle.id)">
                {{ muscle.name }}{{ muscle.is_active ? '' : ' · inactive' }}
              </option>
            </select>
          </label>

          <button
            v-if="hasFilters"
            class="btn btn-ghost btn-sm clear-filters"
            type="button"
            @click="clearFilters"
          >
            <Icon name="close" :size="14" />
            Clear filters
          </button>
        </div>
      </div>

      <div v-if="period === 'custom'" class="custom-range">
        <label class="field">
          <span class="field-label">Start date</span>
          <input v-model="customStart" class="input" type="date" :max="today" />
        </label>
        <span class="range-dash">—</span>
        <label class="field">
          <span class="field-label">End date</span>
          <input v-model="customEnd" class="input" type="date" :max="today" />
        </label>
        <button class="btn btn-primary btn-sm apply-button" type="button" @click="applyCustomRange">
          Apply range
        </button>
      </div>

      <footer class="filter-footer">
        <span><Icon name="calendar" :size="13" /> {{ rangeLabel }}</span>
        <span v-if="report" class="filter-summary">
          {{ report.exercise.total_sets }} recorded sets in view
        </span>
      </footer>
    </section>

    <div v-if="errorMessage" class="notice notice-danger">
      <Icon name="alert" :size="18" class="notice-icon" />
      <span>{{ errorMessage }}</span>
      <button class="btn btn-sm btn-secondary retry-button" type="button" @click="loadReport">
        Retry
      </button>
    </div>

    <template v-if="loading && !report">
      <div class="summary-grid">
        <div v-for="index in 6" :key="index" class="card skeleton summary-skeleton" />
      </div>
      <div class="card skeleton chart-skeleton" />
    </template>

    <template v-else>
      <div class="summary-grid">
        <StatCard
          label="Total volume"
          :value="formatCompact(exerciseStats.total_volume || 0)"
          unit="kg"
          icon="chartBar"
          tone="info"
          hint="Weight × effective reps"
          compact
        />
        <StatCard
          label="Sets"
          :value="exerciseStats.total_sets || 0"
          icon="dumbbell"
          compact
        />
        <StatCard
          label="Effective reps"
          :value="formatNumber(exerciseStats.total_effective_reps || 0, 1)"
          icon="target"
          tone="violet"
          hint="Partials count as half"
          compact
        />
        <StatCard
          label="Heaviest set"
          :value="exerciseStats.max_weight !== null ? formatNumber(exerciseStats.max_weight, 2) : '—'"
          :unit="exerciseStats.max_weight !== null ? 'kg' : ''"
          icon="flame"
          tone="warning"
          compact
        />
        <StatCard
          label="Average weight"
          :value="exerciseStats.average_weight !== null ? formatNumber(exerciseStats.average_weight, 2) : '—'"
          :unit="exerciseStats.average_weight !== null ? 'kg' : ''"
          icon="scale"
          tone="warning"
          compact
        />
        <StatCard
          label="Average RIR"
          :value="exerciseStats.average_rir !== null ? formatNumber(exerciseStats.average_rir, 1) : '—'"
          icon="sparkles"
          hint="Sets missing RIR excluded"
          compact
        />
      </div>

      <section class="chart-card card">
        <header class="chart-header">
          <div class="chart-heading">
            <h2>{{ chartTitle }}</h2>
            <p>{{ chartDescription }}</p>
          </div>
          <div class="chart-tabs segmented">
            <button
              v-for="option in chartOptions"
              :key="option.key"
              class="segment"
              :class="{ active: chartKey === option.key }"
              type="button"
              @click="chartKey = option.key"
            >
              {{ option.label }}
            </button>
          </div>
        </header>

        <div class="chart-body">
          <LineChart
            v-if="chartKey === 'weight'"
            :points="exerciseStats.weight_progression || []"
            unit="kg"
            color="#5eead4"
            :max-decimals="2"
            label="Weight progression by workout date"
          />
          <LineChart
            v-else-if="chartKey === 'volume'"
            :points="exerciseStats.volume_progression || []"
            unit="kg"
            color="#7dd3fc"
            label="Volume progression by workout date"
          />
          <BarChart
            v-else-if="chartKey === 'weekly'"
            :items="weeklyItems"
            unit="kg"
            color="#5eead4"
            label="Weekly training volume"
          />
          <DonutChart
            v-else-if="chartKey === 'muscles'"
            :items="report?.muscle_distribution || []"
            unit="kg"
            label="Attributed volume by muscle group"
          />
          <BarChart
            v-else-if="chartKey === 'frequency'"
            :items="frequencyItems"
            unit="sets"
            horizontal
            color="#c4b5fd"
            :max-decimals="0"
            label="Exercise frequency in sets"
          />
          <LineChart
            v-else
            :points="rirPoints"
            unit="RIR"
            color="#fbbf24"
            :max-decimals="1"
            label="Average RIR trend"
          />
        </div>

        <footer v-if="!hasData" class="chart-empty-footer">
          <Icon name="info" :size="15" />
          No workout records match this period and filter combination.
        </footer>
      </section>

      <div v-if="hasData" class="details-grid">
        <section class="muscle-card card">
          <header class="section-header">
            <div>
              <h2>Muscle group breakdown</h2>
              <p>Primary work counts 100% of set volume; secondary work counts 50%.</p>
            </div>
            <span class="badge">{{ report.muscle_groups.length }} groups</span>
          </header>

          <div class="table-wrap">
            <table class="data-table muscle-table">
              <thead>
                <tr>
                  <th>Muscle group</th>
                  <th>Primary sets</th>
                  <th>Secondary sets</th>
                  <th>Primary volume</th>
                  <th>Secondary volume</th>
                  <th>Attributed</th>
                  <th>Days</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="row in report.muscle_groups" :key="row.muscle_group_id">
                  <td class="muscle-name">{{ row.name }}</td>
                  <td>{{ row.primary_sets }}</td>
                  <td>{{ row.secondary_sets }}</td>
                  <td>{{ formatNumber(row.primary_volume, 1) }} kg</td>
                  <td>{{ formatNumber(row.secondary_volume, 1) }} kg</td>
                  <td class="attributed">{{ formatNumber(row.total_volume, 1) }} kg</td>
                  <td>{{ row.workout_days }}</td>
                </tr>
              </tbody>
            </table>
          </div>
        </section>

        <aside class="method-card card">
          <span class="method-icon">
            <Icon name="info" :size="20" />
          </span>
          <h3>How the numbers work</h3>
          <ul class="method-list">
            <li>
              <strong>Effective reps</strong>
              <span>Complete reps plus partial reps divided by two.</span>
            </li>
            <li>
              <strong>Volume</strong>
              <span>Weight in kilograms multiplied by effective reps.</span>
            </li>
            <li>
              <strong>Muscle volume</strong>
              <span>Primary groups receive 100%; secondary groups receive 50%.</span>
            </li>
            <li>
              <strong>Missing records</strong>
              <span>Dates and weeks without sets stay absent. Nothing is estimated.</span>
            </li>
          </ul>
        </aside>
      </div>

      <EmptyState
        v-else-if="!loading"
        icon="analytics"
        title="No workouts in this view"
        message="Try a wider period, clear a filter, or log some sets to build your analytics."
      >
        <RouterLink class="btn btn-primary" to="/workout">
          <Icon name="plus" :size="16" />
          Log a workout
        </RouterLink>
      </EmptyState>
    </template>
  </div>
</template>

<style scoped>
.filter-panel {
  padding: 16px 16px 0;
  overflow: hidden;
}

.filter-main {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  justify-content: space-between;
  gap: 16px;
}

.period-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.filter-label {
  color: var(--text-soft);
  font-size: 0.75rem;
  font-weight: 600;
}

.select-filters {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: 8px;
}

.compact-field {
  min-width: 170px;
}

.compact-field .select {
  height: 40px;
  min-height: 40px;
  padding-top: 6px;
  padding-bottom: 6px;
  font-size: 0.8125rem;
}

.clear-filters {
  margin-bottom: 2px;
  color: var(--muted);
}

.custom-range {
  display: flex;
  align-items: flex-end;
  gap: 8px;
  margin-top: 16px;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--surface-2);
}

.custom-range .field {
  min-width: 160px;
}

.custom-range .input {
  height: 38px;
  min-height: 38px;
  padding-top: 6px;
  padding-bottom: 6px;
}

.range-dash {
  padding-bottom: 8px;
  color: var(--muted-2);
}

.apply-button {
  margin-bottom: 1px;
}

.filter-footer {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-top: 16px;
  padding: 12px 0;
  border-top: 1px solid var(--divider);
  color: var(--muted-2);
  font-size: 0.75rem;
}

.filter-footer span {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.filter-summary {
  font-family: var(--font-mono);
}

.retry-button {
  margin-left: auto;
}

.summary-grid {
  display: grid;
  grid-template-columns: repeat(6, minmax(0, 1fr));
  gap: 12px;
}

.summary-skeleton {
  height: 105px;
}

.chart-skeleton {
  height: 520px;
}

.chart-card {
  overflow: hidden;
}

.chart-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 20px;
  padding: 20px 20px 16px;
}

.chart-heading {
  max-width: 580px;
}

.chart-heading h2 {
  font-size: 1.125rem;
}

.chart-heading > p:last-child {
  margin-top: 6px;
  color: var(--muted);
  font-size: 0.8125rem;
  line-height: 1.6;
}

.chart-tabs {
  flex: 0 0 auto;
  max-width: 100%;
}

.chart-body {
  padding: 8px 20px 20px;
}

.chart-empty-footer {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0 20px 20px;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface-2);
  color: var(--muted);
  font-size: 0.75rem;
}

.details-grid {
  display: grid;
  grid-template-columns: minmax(0, 2.2fr) minmax(260px, 0.8fr);
  align-items: start;
  gap: 16px;
}

.muscle-card {
  padding: 20px 20px 20px;
  overflow: hidden;
}

.section-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 16px;
}

.section-header p:last-child {
  margin-top: 6px;
  color: var(--muted);
  font-size: 0.75rem;
}

.section-header h2 {
  font-size: 1.125rem;
}

.muscle-table td {
  font-family: var(--font-mono);
  font-size: 0.75rem;
}

.muscle-table .muscle-name {
  color: var(--text);
  font-family: var(--font-sans);
  font-weight: 600;
}

.muscle-table .attributed {
  color: var(--accent);
}

.method-card {
  padding: 20px;
}

.method-icon {
  display: grid;
  width: 40px;
  height: 40px;
  margin-bottom: 12px;
  border: 1px solid var(--accent-line);
  border-radius: 11px;
  background: var(--accent-soft);
  color: var(--accent);
  place-items: center;
}

.method-card h3 {
  font-size: 1rem;
}

.method-list {
  display: flex;
  flex-direction: column;
  gap: 16px;
  margin: 16px 0 0;
  padding: 0;
  list-style: none;
}

.method-list li {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding-left: 12px;
  border-left: 1px solid var(--border-strong);
}

.method-list strong {
  color: var(--text-soft);
  font-size: 0.75rem;
  font-weight: 600;
}

.method-list span {
  color: var(--muted);
  font-size: 0.75rem;
  line-height: 1.55;
}

@media (max-width: 1300px) {
  .summary-grid {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
}

@media (max-width: 1050px) {
  .details-grid {
    grid-template-columns: 1fr;
  }

  .method-card {
    order: -1;
  }
}

@media (max-width: 900px) {
  .filter-main {
    align-items: stretch;
    flex-direction: column;
  }

  .select-filters {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .clear-filters {
    grid-column: 1 / -1;
    justify-self: start;
  }

  .chart-header {
    flex-direction: column;
  }

  .chart-tabs {
    width: 100%;
  }
}

@media (max-width: 700px) {
  .summary-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  .filter-panel {
    padding: 16px 16px 0;
  }

  .period-group {
    min-width: 0;
  }

  .period-group .segmented {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
  }

  .select-filters {
    grid-template-columns: 1fr;
  }

  .custom-range {
    align-items: stretch;
    flex-direction: column;
  }

  .range-dash {
    display: none;
  }

  .custom-range .field {
    min-width: 0;
  }

  .apply-button {
    width: 100%;
  }

  .chart-header {
    padding: 16px 16px 12px;
  }

  .chart-tabs {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
  }

  .chart-body {
    padding: 4px 16px 16px;
  }

  .chart-empty-footer {
    margin: 0 16px 16px;
  }

  .muscle-card {
    padding: 16px;
  }

  .section-header {
    flex-direction: column;
  }
}
</style>
