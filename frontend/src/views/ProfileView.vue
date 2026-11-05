<script setup>
import { computed, onMounted, ref, watch } from 'vue'
import { toast } from 'vue3-toastify'
import EmptyState from '@/components/EmptyState.vue'
import Icon from '@/components/Icon.vue'
import MeasurementModal from '@/components/MeasurementModal.vue'
import PageHeader from '@/components/PageHeader.vue'
import ProfileEditModal from '@/components/ProfileEditModal.vue'
import StatCard from '@/components/StatCard.vue'
import LineChart from '@/components/charts/LineChart.vue'
import { getMetricSeries, getProfile } from '@/api/profile'
import {
  formatDatePretty,
  formatDateShort,
  formatDelta,
  formatNumber,
  initials,
  parseISODate,
} from '@/utils/format'

const metricOptions = [
  { key: 'weight', label: 'Body weight', unit: 'kg', color: '#5eead4', decimals: 1 },
  { key: 'bmi', label: 'BMI', unit: '', color: '#7dd3fc', decimals: 1 },
  { key: 'body-fat', label: 'Body fat', unit: '%', color: '#fbbf24', decimals: 1 },
  { key: 'waist', label: 'Waist', unit: 'cm', color: '#c4b5fd', decimals: 1 },
  { key: 'chest', label: 'Chest', unit: 'cm', color: '#93c5fd', decimals: 1 },
  { key: 'arm', label: 'Arm', unit: 'cm', color: '#f9a8d4', decimals: 1 },
  { key: 'thigh', label: 'Thigh', unit: 'cm', color: '#fdba74', decimals: 1 },
]

const periods = [
  { key: '90', label: '90D' },
  { key: '6m', label: '6M' },
  { key: '12m', label: '1Y' },
  { key: 'all', label: 'All' },
]

const measurementFields = [
  { key: 'weight_kg', label: 'Weight', unit: 'kg' },
  { key: 'height_cm', label: 'Height', unit: 'cm' },
  { key: 'body_fat_percentage', label: 'Body fat', unit: '%' },
  { key: 'waist_cm', label: 'Waist', unit: 'cm' },
  { key: 'chest_cm', label: 'Chest', unit: 'cm' },
  { key: 'hip_cm', label: 'Hip', unit: 'cm' },
  { key: 'arm_cm', label: 'Arm', unit: 'cm' },
  { key: 'thigh_cm', label: 'Thigh', unit: 'cm' },
]

const profile = ref(null)
const series = ref(null)
const loading = ref(true)
const seriesLoading = ref(false)
const errorMessage = ref('')
const selectedMetric = ref('weight')
const selectedPeriod = ref('all')
const editModalOpen = ref(false)
const measurementModalOpen = ref(false)
const showAllMeasurements = ref(false)

const user = computed(() => profile.value?.user || null)
const metrics = computed(() => profile.value?.latest_metrics || {})
const comparisons = computed(() => profile.value?.comparisons || [])
const measurements = computed(() => profile.value?.measurements || [])
const measurementCount = computed(() => profile.value?.measurement_count || 0)
const visibleMeasurements = computed(() =>
  showAllMeasurements.value ? measurements.value : measurements.value.slice(0, 6),
)
const hasMoreMeasurements = computed(() => measurements.value.length > 6)
const currentMetric = computed(
  () => metricOptions.find((metric) => metric.key === selectedMetric.value) || metricOptions[0],
)
const seriesPoints = computed(() => series.value?.points || [])
const avatarInitials = computed(() => initials(user.value?.name))

const age = computed(() => {
  const date = parseISODate(user.value?.date_of_birth)
  if (!date) return null
  const now = new Date()
  let years = now.getFullYear() - date.getFullYear()
  const beforeBirthday =
    now.getMonth() < date.getMonth() ||
    (now.getMonth() === date.getMonth() && now.getDate() < date.getDate())
  if (beforeBirthday) years -= 1
  return years >= 0 ? years : null
})

const latestSnapshot = computed(() => metrics.value.latest_measurement_date || null)

const latestHint = computed(() =>
  latestSnapshot.value ? `Latest snapshot ${formatDatePretty(latestSnapshot.value)}` : 'No snapshots yet',
)

const bmiHint = computed(() => {
  if (metrics.value.bmi === null || metrics.value.bmi === undefined) {
    return 'Needs weight and height in the same snapshot'
  }
  const date = metrics.value.bmi_measured_at
  return date
    ? `${metrics.value.bmi_category} · ${formatDateShort(date)}`
    : metrics.value.bmi_category
})

const metricCards = computed(() => [
  {
    label: 'Weight',
    value: metrics.value.weight_kg !== null && metrics.value.weight_kg !== undefined
      ? formatNumber(metrics.value.weight_kg, 1)
      : '—',
    unit: metrics.value.weight_kg !== null && metrics.value.weight_kg !== undefined ? 'kg' : '',
    icon: 'scale',
    tone: 'accent',
    hint: latestHint.value,
  },
  {
    label: 'BMI',
    value:
      metrics.value.bmi !== null && metrics.value.bmi !== undefined
        ? formatNumber(metrics.value.bmi, 1)
        : '—',
    unit: '',
    icon: 'target',
    tone: 'info',
    hint: bmiHint.value,
  },
  {
    label: 'Body fat',
    value:
      metrics.value.body_fat_percentage !== null &&
      metrics.value.body_fat_percentage !== undefined
        ? formatNumber(metrics.value.body_fat_percentage, 1)
        : '—',
    unit:
      metrics.value.body_fat_percentage !== null &&
      metrics.value.body_fat_percentage !== undefined
        ? '%'
        : '',
    icon: 'flame',
    tone: 'warning',
    hint: 'Latest recorded value',
  },
  {
    label: 'Waist',
    value:
      metrics.value.waist_cm !== null && metrics.value.waist_cm !== undefined
        ? formatNumber(metrics.value.waist_cm, 1)
        : '—',
    unit: metrics.value.waist_cm !== null && metrics.value.waist_cm !== undefined ? 'cm' : '',
    icon: 'ruler',
    tone: 'violet',
    hint: 'Latest recorded value',
  },
  {
    label: 'Chest',
    value:
      metrics.value.chest_cm !== null && metrics.value.chest_cm !== undefined
        ? formatNumber(metrics.value.chest_cm, 1)
        : '—',
    unit: metrics.value.chest_cm !== null && metrics.value.chest_cm !== undefined ? 'cm' : '',
    icon: 'shield',
    tone: 'info',
    hint: 'Latest recorded value',
  },
  {
    label: 'Hip',
    value:
      metrics.value.hip_cm !== null && metrics.value.hip_cm !== undefined
        ? formatNumber(metrics.value.hip_cm, 1)
        : '—',
    unit: metrics.value.hip_cm !== null && metrics.value.hip_cm !== undefined ? 'cm' : '',
    icon: 'circle',
    tone: 'accent',
    hint: 'Latest recorded value',
  },
  {
    label: 'Arm',
    value:
      metrics.value.arm_cm !== null && metrics.value.arm_cm !== undefined
        ? formatNumber(metrics.value.arm_cm, 1)
        : '—',
    unit: metrics.value.arm_cm !== null && metrics.value.arm_cm !== undefined ? 'cm' : '',
    icon: 'dumbbell',
    tone: 'warning',
    hint: 'Latest recorded value',
  },
  {
    label: 'Thigh',
    value:
      metrics.value.thigh_cm !== null && metrics.value.thigh_cm !== undefined
        ? formatNumber(metrics.value.thigh_cm, 1)
        : '—',
    unit: metrics.value.thigh_cm !== null && metrics.value.thigh_cm !== undefined ? 'cm' : '',
    icon: 'flame',
    tone: 'danger',
    hint: 'Latest recorded value',
  },
])

const chartDescription = computed(() => {
  return `${currentMetric.value.label} from actual measurement snapshots. Dates without a record are never filled in.`
})

const chartRangeLabel = computed(() => {
  if (!series.value) return ''
  return series.value.period_label || ''
})

function metricDisplay(value, unit) {
  if (value === null || value === undefined) return '—'
  return `${formatNumber(value, 2)}${unit ? ` ${unit}` : ''}`
}

function comparisonDates(row) {
  if (!row.current_date && !row.previous_date) return 'Not recorded'
  if (row.current_date && row.previous_date) {
    return `${formatDateShort(row.previous_date)} → ${formatDateShort(row.current_date)}`
  }
  return row.current_date
    ? `Only ${formatDateShort(row.current_date)} recorded`
    : `Only ${formatDateShort(row.previous_date)} recorded`
}

function measurementValues(measurement) {
  return measurementFields
    .filter(
      (field) =>
        measurement[field.key] !== null && measurement[field.key] !== undefined,
    )
    .map((field) => ({
      label: field.label,
      value: `${formatNumber(measurement[field.key], 1)} ${field.unit}`,
    }))
}

async function loadProfile({ silent = false } = {}) {
  if (!silent) loading.value = true
  errorMessage.value = ''
  try {
    profile.value = await getProfile()
  } catch (error) {
    errorMessage.value = error.userMessage || 'Could not load your profile.'
  } finally {
    loading.value = false
  }
}

async function loadSeries() {
  seriesLoading.value = true
  try {
    series.value = await getMetricSeries({
      metric: selectedMetric.value,
      period: selectedPeriod.value,
    })
  } catch (error) {
    toast.error(error.userMessage || 'Could not load the measurement chart.')
  } finally {
    seriesLoading.value = false
  }
}

async function handleMeasurementSaved() {
  await loadProfile({ silent: true })
  await loadSeries()
}

async function handleProfileSaved() {
  await loadProfile({ silent: true })
}

watch([selectedMetric, selectedPeriod], loadSeries)

onMounted(async () => {
  await loadProfile()
  if (!errorMessage.value) await loadSeries()
})
</script>

<template>
  <div class="page">
    <PageHeader
      eyebrow="Athlete profile"
      title="Profile"
      description="Your local identity and body measurement record. Snapshots are append-only and BMI is only derived from values recorded together."
    >
      <template #actions>
        <button class="btn btn-secondary" type="button" @click="editModalOpen = true">
          <Icon name="edit" :size="16" />
          Edit profile
        </button>
        <button class="btn btn-primary" type="button" @click="measurementModalOpen = true">
          <Icon name="plus" :size="16" />
          Add measurement
        </button>
      </template>
    </PageHeader>

    <div v-if="errorMessage" class="notice notice-danger">
      <Icon name="alert" :size="18" class="notice-icon" />
      <span>{{ errorMessage }}</span>
      <button class="btn btn-sm btn-secondary retry-button" type="button" @click="loadProfile()">
        Retry
      </button>
    </div>

    <template v-if="loading && !profile">
      <div class="card skeleton hero-skeleton" />
      <div class="metric-grid">
        <div v-for="index in 8" :key="index" class="card skeleton metric-skeleton" />
      </div>
      <div class="analysis-grid">
        <div class="card skeleton chart-skeleton" />
        <div class="card skeleton comparison-skeleton" />
      </div>
    </template>

    <template v-else-if="profile">
      <section class="profile-hero card">
        <div class="avatar" aria-hidden="true">{{ avatarInitials }}</div>

        <div class="hero-main">
          <div class="hero-title">
            <h2>{{ user.name }}</h2>
            <span class="badge badge-accent">Local profile</span>
          </div>
          <p class="hero-subtitle">
            Single-user tracking on this server. No account, no cloud sync — just your records.
          </p>
          <div class="hero-meta">
            <span class="chip">
              <Icon name="calendar" :size="13" />
              <template v-if="user.date_of_birth">
                Born {{ formatDatePretty(user.date_of_birth) }}
                <template v-if="age !== null"> · {{ age }} years</template>
              </template>
              <template v-else>Date of birth not set</template>
            </span>
            <span class="chip">
              <Icon name="ruler" :size="13" />
              {{ user.height_cm ? `${formatNumber(user.height_cm, 1)} cm profile height` : 'Profile height not set' }}
            </span>
            <span class="chip">
              <Icon name="database" :size="13" />
              {{ measurementCount }} {{ measurementCount === 1 ? 'snapshot' : 'snapshots' }}
            </span>
          </div>
        </div>

        <div class="hero-stats">
          <div class="hero-stat">
            <span>Latest snapshot</span>
            <strong>{{ latestSnapshot ? formatDatePretty(latestSnapshot) : '—' }}</strong>
          </div>
          <div class="hero-stat">
            <span>Profile height</span>
            <strong>{{ user.height_cm ? `${formatNumber(user.height_cm, 1)} cm` : '—' }}</strong>
          </div>
        </div>
      </section>

      <section class="metrics-section">
        <header class="section-heading">
          <div>
            <h2>Latest measurements</h2>
            <p>Each metric uses its own most recent observation; missing values stay blank.</p>
          </div>
          <span class="badge">{{ latestSnapshot ? formatDatePretty(latestSnapshot) : 'No data' }}</span>
        </header>

        <div class="metric-grid">
          <StatCard
            v-for="card in metricCards"
            :key="card.label"
            :label="card.label"
            :value="card.value"
            :unit="card.unit"
            :icon="card.icon"
            :tone="card.tone"
            :hint="card.hint"
            compact
          />
        </div>
      </section>

      <div class="analysis-grid">
        <section class="chart-card card">
          <header class="chart-header">
            <div class="chart-heading">
              <p class="eyebrow">Measurement trend</p>
              <h2>{{ currentMetric.label }} over time</h2>
              <p>{{ chartDescription }}</p>
            </div>
            <div class="chart-controls">
              <label class="field metric-field">
                <span class="field-label">Metric</span>
                <select v-model="selectedMetric" class="select">
                  <option v-for="metric in metricOptions" :key="metric.key" :value="metric.key">
                    {{ metric.label }}
                  </option>
                </select>
              </label>
              <div class="segmented period-tabs">
                <button
                  v-for="period in periods"
                  :key="period.key"
                  class="segment"
                  :class="{ active: selectedPeriod === period.key }"
                  type="button"
                  @click="selectedPeriod = period.key"
                >
                  {{ period.label }}
                </button>
              </div>
            </div>
          </header>

          <div class="chart-body">
            <div v-if="seriesLoading && !seriesPoints.length" class="chart-loading skeleton" />
            <LineChart
              v-else
              :points="seriesPoints"
              :unit="currentMetric.unit"
              :color="currentMetric.color"
              :max-decimals="currentMetric.decimals"
              :label="`${currentMetric.label} measurement trend`"
            />
          </div>

          <footer class="chart-footer">
            <span class="chart-range">
              <Icon name="calendar" :size="13" />
              {{ chartRangeLabel || 'Recorded dates only' }}
            </span>
            <span v-if="seriesPoints.length" class="badge">
              {{ seriesPoints.length }} {{ seriesPoints.length === 1 ? 'point' : 'points' }}
            </span>
          </footer>
        </section>

        <aside class="comparison-card card">
          <header class="comparison-header">
            <p class="eyebrow">Delta</p>
            <h2>Changes since previous observation</h2>
            <p>Each metric compares its two most recent recorded values, independently.</p>
          </header>

          <div v-if="comparisons.length" class="comparison-list">
            <div class="comparison-head">
              <span>Metric</span>
              <span>Current</span>
              <span>Change</span>
              <span>Dates</span>
            </div>
            <div v-for="row in comparisons" :key="row.metric" class="comparison-row">
              <span class="comparison-label">{{ row.label }}</span>
              <span class="comparison-current">{{ metricDisplay(row.current, row.unit) }}</span>
              <span
                class="comparison-delta"
                :class="{
                  positive: row.delta !== null && row.delta > 0,
                  negative: row.delta !== null && row.delta < 0,
                }"
              >
                <template v-if="row.delta !== null">
                  <Icon
                    :name="row.delta > 0 ? 'arrowUp' : row.delta < 0 ? 'arrowDown' : 'circle'"
                    :size="12"
                  />
                  {{ formatDelta(row.delta, row.unit) }}
                </template>
                <template v-else>—</template>
              </span>
              <span class="comparison-dates">{{ comparisonDates(row) }}</span>
            </div>
          </div>

          <p v-else class="comparison-empty">No measurements recorded yet.</p>

          <div class="comparison-note">
            <Icon name="info" :size="14" />
            <span>A comparison appears only when a metric has at least two observations.</span>
          </div>
        </aside>
      </div>

      <section class="history-card card">
        <header class="history-header">
          <div>
            <h2>Measurement history</h2>
            <p>
              Snapshots are append-only. BMI is only derived from weight and height recorded in
              the same snapshot.
            </p>
          </div>
          <div class="history-actions">
            <span class="badge">{{ measurementCount }} total</span>
            <button class="btn btn-secondary btn-sm" type="button" @click="measurementModalOpen = true">
              <Icon name="plus" :size="14" />
              Add snapshot
            </button>
          </div>
        </header>

        <div v-if="visibleMeasurements.length" class="measurement-list">
          <article
            v-for="measurement in visibleMeasurements"
            :key="measurement.id"
            class="measurement-row"
          >
            <div class="measurement-date">
              <span class="measurement-day">
                {{ formatDateShort(measurement.measured_at).split(' ')[0] }}
              </span>
              <span class="measurement-month">
                {{ formatDateShort(measurement.measured_at).split(' ')[1] }}
              </span>
            </div>

            <div class="measurement-main">
              <div class="measurement-topline">
                <h3>{{ formatDatePretty(measurement.measured_at) }}</h3>
                <span v-if="measurement.bmi !== null" class="badge badge-accent">
                  BMI {{ formatNumber(measurement.bmi, 1) }} · {{ measurement.bmi_category }}
                </span>
                <span v-else class="badge badge-muted">BMI unavailable</span>
              </div>

              <div v-if="measurementValues(measurement).length" class="measurement-values">
                <span
                  v-for="value in measurementValues(measurement)"
                  :key="value.label"
                  class="measurement-value"
                >
                  <span>{{ value.label }}</span>
                  <strong>{{ value.value }}</strong>
                </span>
              </div>
              <p v-else class="measurement-empty">Note only — no numeric values recorded.</p>

              <p v-if="measurement.notes" class="measurement-notes">
                <Icon name="edit" :size="12" />
                {{ measurement.notes }}
              </p>
            </div>
          </article>
        </div>

        <EmptyState
          v-else
          icon="scale"
          title="No body measurements yet"
          message="Add a snapshot with whatever you measured. You can record weight, height, body fat, girths, or just a note."
        >
          <button class="btn btn-primary" type="button" @click="measurementModalOpen = true">
            <Icon name="plus" :size="16" />
            Add your first snapshot
          </button>
        </EmptyState>

        <footer v-if="hasMoreMeasurements" class="history-footer">
          <button
            class="btn btn-ghost btn-sm"
            type="button"
            @click="showAllMeasurements = !showAllMeasurements"
          >
            <Icon :name="showAllMeasurements ? 'chevronLeft' : 'chevronRight'" :size="14" />
            {{ showAllMeasurements ? 'Show fewer snapshots' : `Show all ${measurementCount} snapshots` }}
          </button>
        </footer>
      </section>
    </template>

    <ProfileEditModal
      :open="editModalOpen"
      :user="user"
      @close="editModalOpen = false"
      @saved="handleProfileSaved"
    />

    <MeasurementModal
      :open="measurementModalOpen"
      @close="measurementModalOpen = false"
      @saved="handleMeasurementSaved"
    />
  </div>
</template>

<style scoped>
.retry-button {
  margin-left: auto;
}

.hero-skeleton {
  height: 168px;
}

.metric-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 12px;
}

.metric-skeleton {
  height: 92px;
}

.profile-hero {
  position: relative;
  display: flex;
  align-items: center;
  gap: 20px;
  padding: 24px;
  overflow: hidden;
}

.profile-hero::after {
  position: absolute;
  top: -80px;
  right: -40px;
  width: 260px;
  height: 260px;
  border-radius: 50%;
  background: radial-gradient(circle, rgba(94, 234, 212, 0.12), transparent 68%);
  content: "";
  pointer-events: none;
}

.avatar {
  display: grid;
  width: 76px;
  height: 76px;
  flex: 0 0 auto;
  border: 1px solid var(--accent-line);
  border-radius: 22px;
  background:
    linear-gradient(145deg, rgba(94, 234, 212, 0.18), rgba(125, 211, 252, 0.06)),
    var(--surface-2);
  color: var(--accent);
  font-family: var(--font-mono);
  font-size: 1.45rem;
  font-weight: 600;
  letter-spacing: 0.04em;
  place-items: center;
}

.hero-main {
  min-width: 0;
  flex: 1;
}

.hero-title {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 9px;
}

.hero-title h2 {
  font-size: 1.35rem;
}

.hero-subtitle {
  margin-top: 6px;
  color: var(--muted);
  font-size: 0.79rem;
  line-height: 1.55;
}

.hero-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 10px;
}

.hero-stats {
  position: relative;
  z-index: 1;
  display: flex;
  flex: 0 0 auto;
  gap: 24px;
  padding-left: 24px;
  border-left: 1px solid var(--border);
}

.hero-stat {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.hero-stat span {
  color: var(--muted-2);
  font-size: 0.63rem;
  font-weight: 650;
  letter-spacing: 0.07em;
  text-transform: uppercase;
}

.hero-stat strong {
  color: var(--text-soft);
  font-family: var(--font-mono);
  font-size: 0.83rem;
  font-weight: 550;
  white-space: nowrap;
}

.metrics-section {
  display: flex;
  flex-direction: column;
  gap: 13px;
}

.analysis-grid {
  display: grid;
  grid-template-columns: minmax(0, 1.6fr) minmax(340px, 1fr);
  align-items: start;
  gap: 16px;
}

.chart-card,
.comparison-card {
  overflow: hidden;
}

.chart-header {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  justify-content: space-between;
  gap: 18px;
  padding: 21px 22px 15px;
}

.chart-heading {
  max-width: 440px;
}

.chart-heading h2 {
  font-size: 1.1rem;
}

.chart-heading > p:last-child {
  margin-top: 6px;
  color: var(--muted);
  font-size: 0.76rem;
  line-height: 1.6;
}

.chart-controls {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: 9px;
}

.metric-field {
  min-width: 140px;
}

.metric-field .select {
  height: 38px;
  min-height: 38px;
  padding-top: 5px;
  padding-bottom: 5px;
  font-size: 0.78rem;
}

.period-tabs {
  padding: 3px;
}

.period-tabs .segment {
  min-height: 31px;
  padding: 5px 9px;
  font-size: 0.7rem;
}

.chart-body {
  padding: 2px 22px 16px;
}

.chart-loading {
  height: 250px;
  border-radius: var(--radius);
}

.chart-skeleton {
  height: 460px;
}

.chart-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin: 0 22px;
  padding: 12px 0 16px;
  border-top: 1px solid var(--border);
  color: var(--muted-2);
  font-size: 0.67rem;
}

.chart-range {
  display: inline-flex;
  align-items: center;
  gap: 5px;
}

.comparison-skeleton {
  height: 460px;
}

.comparison-header {
  padding: 21px 22px 15px;
  border-bottom: 1px solid var(--border);
}

.comparison-header h2 {
  font-size: 1.05rem;
}

.comparison-header p:last-child {
  margin-top: 6px;
  color: var(--muted);
  font-size: 0.73rem;
  line-height: 1.55;
}

.comparison-list {
  padding: 8px 22px 5px;
}

.comparison-head,
.comparison-row {
  display: grid;
  grid-template-columns: minmax(70px, 0.9fr) minmax(74px, 0.9fr) minmax(86px, 1fr) minmax(110px, 1.35fr);
  align-items: center;
  gap: 8px;
}

.comparison-head {
  padding: 9px 0 7px;
  color: var(--muted-2);
  font-size: 0.57rem;
  font-weight: 700;
  letter-spacing: 0.07em;
  text-transform: uppercase;
}

.comparison-row {
  min-height: 36px;
  padding: 7px 0;
  border-top: 1px solid rgba(38, 50, 67, 0.62);
}

.comparison-label {
  color: var(--text-soft);
  font-size: 0.75rem;
  font-weight: 550;
}

.comparison-current {
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 0.72rem;
  font-weight: 550;
  white-space: nowrap;
}

.comparison-delta {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  min-width: 0;
  color: var(--muted-2);
  font-family: var(--font-mono);
  font-size: 0.68rem;
  white-space: nowrap;
}

.comparison-delta.positive {
  color: var(--accent);
}

.comparison-delta.negative {
  color: var(--info);
}

.comparison-dates {
  overflow: hidden;
  color: var(--muted-2);
  font-family: var(--font-mono);
  font-size: 0.62rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.comparison-empty {
  padding: 24px 22px;
  color: var(--muted);
  font-size: 0.78rem;
}

.comparison-note {
  display: flex;
  align-items: flex-start;
  gap: 7px;
  margin: 10px 22px 18px;
  padding: 10px 11px;
  border: 1px solid var(--border);
  border-radius: 9px;
  background: var(--surface-2);
  color: var(--muted-2);
  font-size: 0.67rem;
  line-height: 1.5;
}

.comparison-note :deep(svg) {
  flex: 0 0 auto;
  margin-top: 1px;
  color: var(--info);
}

.history-card {
  overflow: hidden;
}

.history-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 18px;
  padding: 21px 22px;
  border-bottom: 1px solid var(--border);
}

.history-header h2 {
  font-size: 1.08rem;
}

.history-header p {
  max-width: 650px;
  margin-top: 6px;
  color: var(--muted);
  font-size: 0.76rem;
  line-height: 1.55;
}

.history-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}

.measurement-list {
  display: flex;
  flex-direction: column;
}

.measurement-row {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  padding: 16px 22px;
  border-bottom: 1px solid var(--border);
}

.measurement-row:last-child {
  border-bottom: 0;
}

.measurement-date {
  display: flex;
  width: 48px;
  height: 51px;
  flex: 0 0 auto;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border);
  border-radius: 11px;
  background: var(--surface-2);
  line-height: 1;
}

.measurement-day {
  color: var(--accent);
  font-family: var(--font-mono);
  font-size: 1rem;
  font-weight: 600;
}

.measurement-month {
  margin-top: 4px;
  color: var(--muted-2);
  font-size: 0.55rem;
  font-weight: 650;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.measurement-main {
  min-width: 0;
  flex: 1;
}

.measurement-topline {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}

.measurement-topline h3 {
  font-size: 0.85rem;
  font-weight: 600;
}

.measurement-values {
  display: flex;
  flex-wrap: wrap;
  gap: 7px;
  margin-top: 9px;
}

.measurement-value {
  display: inline-flex;
  align-items: baseline;
  gap: 5px;
  min-height: 26px;
  padding: 3px 9px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface-2);
}

.measurement-value span {
  color: var(--muted-2);
  font-size: 0.6rem;
  font-weight: 650;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.measurement-value strong {
  color: var(--text-soft);
  font-family: var(--font-mono);
  font-size: 0.7rem;
  font-weight: 550;
}

.measurement-empty {
  margin-top: 7px;
  color: var(--muted-2);
  font-size: 0.7rem;
}

.measurement-notes {
  display: flex;
  align-items: flex-start;
  gap: 5px;
  margin-top: 9px;
  padding-left: 9px;
  border-left: 2px solid var(--border-strong);
  color: var(--muted);
  font-size: 0.72rem;
  line-height: 1.55;
}

.measurement-notes :deep(svg) {
  flex: 0 0 auto;
  margin-top: 2px;
}

.history-footer {
  display: flex;
  justify-content: center;
  padding: 13px;
  border-top: 1px solid var(--border);
}

@media (max-width: 1200px) {
  .analysis-grid {
    grid-template-columns: 1fr;
  }

  .comparison-card {
    order: 2;
  }
}

@media (max-width: 1000px) {
  .metric-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .profile-hero {
    flex-wrap: wrap;
  }

  .hero-stats {
    width: 100%;
    padding-top: 16px;
    padding-left: 0;
    border-top: 1px solid var(--border);
    border-left: 0;
  }

  .hero-stat {
    flex: 1;
  }
}

@media (max-width: 760px) {
  .metric-grid {
    gap: 9px;
  }

  .profile-hero {
    gap: 14px;
    padding: 18px;
  }

  .avatar {
    width: 58px;
    height: 58px;
    border-radius: 17px;
    font-size: 1.1rem;
  }

  .hero-title h2 {
    font-size: 1.15rem;
  }

  .hero-stats {
    gap: 14px;
  }

  .hero-stat strong {
    font-size: 0.76rem;
    white-space: normal;
  }

  .chart-header {
    flex-direction: column;
    padding: 18px 17px 13px;
  }

  .chart-controls {
    width: 100%;
  }

  .metric-field {
    flex: 1;
  }

  .chart-body {
    padding: 2px 15px 12px;
  }

  .chart-footer {
    margin: 0 17px;
  }

  .comparison-header {
    padding: 18px 17px 13px;
  }

  .comparison-list {
    padding: 6px 17px 4px;
  }

  .comparison-head,
  .comparison-row {
    grid-template-columns: minmax(64px, 0.9fr) minmax(66px, 0.9fr) minmax(74px, 1fr);
  }

  .comparison-head span:last-child,
  .comparison-dates {
    display: none;
  }

  .comparison-note {
    margin-right: 17px;
    margin-left: 17px;
  }

  .history-header {
    flex-direction: column;
    padding: 17px;
  }

  .measurement-row {
    gap: 11px;
    padding: 14px 17px;
  }

  .measurement-date {
    width: 42px;
    height: 46px;
  }
}
</style>
