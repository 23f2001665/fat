<script setup>
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { toast } from 'vue3-toastify'
import EmptyState from '@/components/EmptyState.vue'
import Icon from '@/components/Icon.vue'
import ChangePasswordModal from '@/components/ChangePasswordModal.vue'
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

const route = useRoute()
const router = useRouter()
const profile = ref(null)
const series = ref(null)
const loading = ref(true)
const seriesLoading = ref(false)
const errorMessage = ref('')
const selectedMetric = ref('weight')
const selectedPeriod = ref('all')
const editModalOpen = ref(false)
const measurementModalOpen = ref(false)
const passwordModalOpen = ref(false)
const showAllMeasurements = ref(false)

const user = computed(() => profile.value?.user || null)
const metrics = computed(() => profile.value?.latest_metrics || {})
const comparisons = computed(() => profile.value?.comparisons || [])
const measurements = computed(() => profile.value?.measurements || [])
const hasComparisonData = computed(() =>
  comparisons.value.some((row) => row.current !== null && row.current !== undefined),
)
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

// One icon + tone per kind of metric so cards read consistently:
// body composition metrics get their own icons, all girths share the ruler.
const metricCardDefs = [
  { key: 'weight_kg', label: 'Weight', unit: 'kg', icon: 'scale', tone: 'accent' },
  { key: 'bmi', label: 'BMI', unit: '', icon: 'target', tone: 'info' },
  { key: 'body_fat_percentage', label: 'Body fat', unit: '%', icon: 'flame', tone: 'warning' },
  { key: 'waist_cm', label: 'Waist', unit: 'cm', icon: 'ruler', tone: 'violet' },
  { key: 'chest_cm', label: 'Chest', unit: 'cm', icon: 'ruler', tone: 'violet' },
  { key: 'hip_cm', label: 'Hip', unit: 'cm', icon: 'ruler', tone: 'violet' },
  { key: 'arm_cm', label: 'Arm', unit: 'cm', icon: 'ruler', tone: 'violet' },
  { key: 'thigh_cm', label: 'Thigh', unit: 'cm', icon: 'ruler', tone: 'violet' },
]

function hasValue(value) {
  return value !== null && value !== undefined
}

const metricCards = computed(() =>
  metricCardDefs
    .filter((def) => hasValue(metrics.value[def.key]))
    .map((def) => ({
      ...def,
      value: formatNumber(metrics.value[def.key], 1),
      hint: def.key === 'bmi' ? bmiHint.value : def.key === 'weight_kg' ? latestHint.value : '',
    })),
)

const untrackedMetrics = computed(() =>
  metricCardDefs.filter((def) => !hasValue(metrics.value[def.key])).map((def) => def.label),
)

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
  // Deep link from the getting-started checklist opens the measurement dialog.
  if (route.query.add === 'measurement') {
    measurementModalOpen.value = true
    router.replace({ name: 'profile' })
  }
  await loadProfile()
  if (!errorMessage.value) await loadSeries()
})
</script>

<template>
  <div class="page">
    <PageHeader
      title="Profile"
      description="Your account and body measurements, private to you."
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
            <span class="badge badge-accent">
              {{ user.role === 'admin' ? 'Administrator' : 'Member' }}
            </span>
          </div>
          <p class="hero-subtitle">@{{ user.username }}</p>
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

        <div class="hero-actions">
          <button class="btn btn-ghost btn-sm" type="button" @click="passwordModalOpen = true">
            <Icon name="key" :size="14" />
            Change password
          </button>
        </div>
      </section>

      <section class="metrics-section">
        <header class="section-heading">
          <div>
            <h2>Latest measurements</h2>
            <p>Most recent value of each metric.</p>
          </div>
        </header>

        <div v-if="metricCards.length" class="metric-grid">
          <StatCard
            v-for="card in metricCards"
            :key="card.key"
            :label="card.label"
            :value="card.value"
            :unit="card.unit"
            :icon="card.icon"
            :tone="card.tone"
            :hint="card.hint"
            compact
          />
        </div>
        <p v-if="untrackedMetrics.length" class="untracked-line">
          <span>Not tracked yet: {{ untrackedMetrics.join(', ') }}</span>
          <button class="btn btn-ghost btn-sm" type="button" @click="measurementModalOpen = true">
            <Icon name="plus" :size="14" />
            Add measurement
          </button>
        </p>
      </section>

      <div class="analysis-grid">
        <section class="chart-card card">
          <header class="chart-header">
            <div class="chart-heading">
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
            <h2>Changes since previous observation</h2>
            <p>Each metric compares its two most recent recorded values, independently.</p>
          </header>

          <div v-if="hasComparisonData" class="comparison-list">
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
            <Icon :name="showAllMeasurements ? 'arrowUp' : 'arrowDown'" :size="14" />
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

    <ChangePasswordModal :open="passwordModalOpen" @close="passwordModalOpen = false" />

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
  grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
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
  font-size: 1.5rem;
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
  gap: 8px;
}

.hero-title h2 {
  font-size: 1.5rem;
}

.hero-subtitle {
  margin-top: 6px;
  color: var(--muted);
  font-size: 0.8125rem;
  line-height: 1.55;
}

.hero-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 8px;
}

.hero-actions {
  position: relative;
  z-index: 1;
  display: flex;
  align-self: flex-start;
}

.metrics-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
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
  gap: 16px;
  padding: 20px 20px 16px;
}

.chart-heading {
  max-width: 440px;
}

.chart-heading h2 {
  font-size: 1.125rem;
}

.chart-heading > p:last-child {
  margin-top: 6px;
  color: var(--muted);
  font-size: 0.75rem;
  line-height: 1.6;
}

.chart-controls {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: 8px;
}

.metric-field {
  min-width: 140px;
}

.metric-field .select {
  height: 38px;
  min-height: 38px;
  padding-top: 4px;
  padding-bottom: 4px;
  font-size: 0.8125rem;
}

.period-tabs {
  padding: 4px;
}

.period-tabs .segment {
  min-height: 31px;
  padding: 4px 8px;
  font-size: 0.75rem;
}

.chart-body {
  padding: 2px 20px 16px;
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
  margin: 0 20px;
  padding: 12px 0 16px;
  border-top: 1px solid var(--divider);
  color: var(--muted-2);
  font-size: 0.75rem;
}

.chart-range {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.comparison-skeleton {
  height: 460px;
}

.comparison-header {
  padding: 20px 20px 16px;
  border-bottom: 1px solid var(--divider);
}

.comparison-header h2 {
  font-size: 1rem;
}

.comparison-header p:last-child {
  margin-top: 6px;
  color: var(--muted);
  font-size: 0.75rem;
  line-height: 1.55;
}

.comparison-list {
  padding: 8px 20px 4px;
}

.comparison-head,
.comparison-row {
  display: grid;
  grid-template-columns: minmax(70px, 0.9fr) minmax(74px, 0.9fr) minmax(86px, 1fr) minmax(110px, 1.35fr);
  align-items: center;
  gap: 8px;
}

.comparison-head {
  padding: 8px 0 8px;
  color: var(--muted-2);
  font-size: 0.75rem;
  font-weight: 500;
}

.comparison-row {
  min-height: 36px;
  padding: 8px 0;
  border-top: 1px solid rgba(38, 50, 67, 0.62);
}

.comparison-label {
  color: var(--text-soft);
  font-size: 0.75rem;
  font-weight: 500;
}

.comparison-current {
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 0.75rem;
  font-weight: 500;
  white-space: nowrap;
}

.comparison-delta {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
  color: var(--muted-2);
  font-family: var(--font-mono);
  font-size: 0.75rem;
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
  font-size: 0.75rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.comparison-empty {
  padding: 24px 20px;
  color: var(--muted);
  font-size: 0.8125rem;
}

.comparison-note {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin: 8px 20px 16px;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 9px;
  background: var(--surface-2);
  color: var(--muted-2);
  font-size: 0.75rem;
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
  gap: 16px;
  padding: 20px 20px;
  border-bottom: 1px solid var(--divider);
}

.history-header h2 {
  font-size: 1.125rem;
}

.history-header p {
  max-width: 650px;
  margin-top: 6px;
  color: var(--muted);
  font-size: 0.75rem;
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
  gap: 12px;
  padding: 16px 20px;
  border-bottom: 1px solid var(--divider);
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
  font-size: 0.75rem;
  font-weight: 500;
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
  font-size: 0.875rem;
  font-weight: 600;
}

.measurement-values {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 8px;
}

.measurement-value {
  display: inline-flex;
  align-items: baseline;
  gap: 4px;
  min-height: 26px;
  padding: 4px 8px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--surface-2);
}

.measurement-value span {
  color: var(--muted-2);
  font-size: 0.75rem;
  font-weight: 500;
}

.measurement-value strong {
  color: var(--text-soft);
  font-family: var(--font-mono);
  font-size: 0.75rem;
  font-weight: 500;
}

.measurement-empty {
  margin-top: 8px;
  color: var(--muted-2);
  font-size: 0.75rem;
}

.measurement-notes {
  display: flex;
  align-items: flex-start;
  gap: 4px;
  margin-top: 8px;
  padding-left: 8px;
  border-left: 2px solid var(--border-strong);
  color: var(--muted);
  font-size: 0.75rem;
  line-height: 1.55;
}

.measurement-notes :deep(svg) {
  flex: 0 0 auto;
  margin-top: 2px;
}

.history-footer {
  display: flex;
  justify-content: center;
  padding: 12px;
  border-top: 1px solid var(--divider);
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

  .hero-main {
    flex: 1 1 220px;
  }

  /* Account actions drop below the identity block instead of squeezing it. */
  .hero-actions {
    width: 100%;
    padding-top: 12px;
    border-top: 1px solid var(--divider);
  }
}

@media (max-width: 760px) {
  .metric-grid {
    gap: 8px;
  }

  .profile-hero {
    gap: 12px;
    padding: 16px;
  }

  .avatar {
    width: 58px;
    height: 58px;
    border-radius: 17px;
    font-size: 1.125rem;
  }

  .hero-title h2 {
    font-size: 1.125rem;
  }

  .chart-header {
    flex-direction: column;
    padding: 16px 16px 12px;
  }

  .chart-controls {
    width: 100%;
  }

  .metric-field {
    flex: 1;
  }

  .chart-body {
    padding: 2px 16px 12px;
  }

  .chart-footer {
    margin: 0 16px;
  }

  .comparison-header {
    padding: 16px 16px 12px;
  }

  .comparison-list {
    padding: 6px 16px 4px;
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
    margin-right: 16px;
    margin-left: 16px;
  }

  .history-header {
    flex-direction: column;
    padding: 16px;
  }

  .measurement-row {
    gap: 12px;
    padding: 12px 16px;
  }

  .measurement-date {
    width: 42px;
    height: 46px;
  }
}

.untracked-line {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-top: 12px;
  padding: 8px 8px 8px 16px;
  border: 1px dashed var(--border);
  border-radius: var(--radius);
  color: var(--muted);
  font-size: var(--text-sm);
}
</style>
