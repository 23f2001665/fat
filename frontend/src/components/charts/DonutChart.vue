<script setup>
import { computed } from 'vue'
import { formatNumber } from '@/utils/format'

const props = defineProps({
  items: { type: Array, default: () => [] },
  unit: { type: String, default: '' },
  label: { type: String, default: 'Distribution' },
})

const colors = [
  '#5eead4',
  '#7dd3fc',
  '#c4b5fd',
  '#fbbf24',
  '#f9a8d4',
  '#93c5fd',
  '#fdba74',
  '#86efac',
  '#fda4af',
  '#a5b4fc',
  '#67e8f9',
  '#fde68a',
  '#d8b4fe',
  '#94a3b8',
]

const rows = computed(() => {
  const positive = props.items
    .map((item) => ({ ...item, value: Number(item.value) }))
    .filter((item) => Number.isFinite(item.value) && item.value > 0)
    .sort((a, b) => b.value - a.value)
  const total = positive.reduce((sum, item) => sum + item.value, 0)
  let cursor = 0
  return positive.map((item, index) => {
    const percent = total ? (item.value / total) * 100 : 0
    const segment = {
      ...item,
      percent,
      color: colors[index % colors.length],
      start: cursor,
      end: cursor + percent,
    }
    cursor += percent
    return segment
  })
})

const total = computed(() => rows.value.reduce((sum, item) => sum + item.value, 0))

const gradient = computed(() => {
  if (!rows.value.length) return 'transparent'
  return `conic-gradient(${rows.value
    .map((row) => `${row.color} ${row.start}% ${row.end}%`)
    .join(', ')})`
})

function formatValue(value) {
  return `${formatNumber(value, 1)}${props.unit ? ` ${props.unit}` : ''}`
}
</script>

<template>
  <div v-if="!rows.length" class="chart-empty">
    <span>No recorded volume in this range.</span>
  </div>

  <div v-else class="donut-layout">
    <div class="donut-wrap" role="img" :aria-label="label">
      <div class="donut" :style="{ background: gradient }" />
      <div class="donut-center">
        <strong>{{ formatNumber(total, 1) }}</strong>
        <span>{{ unit || 'total' }}</span>
      </div>
    </div>

    <div class="legend">
      <div v-for="row in rows" :key="row.label" class="legend-item">
        <span class="legend-dot" :style="{ background: row.color }" />
        <span class="legend-label" :title="row.label">{{ row.label }}</span>
        <span class="legend-value">{{ formatValue(row.value) }}</span>
        <span class="legend-percent">{{ formatNumber(row.percent, 0) }}%</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.donut-layout {
  display: grid;
  grid-template-columns: minmax(180px, 0.8fr) minmax(240px, 1.2fr);
  align-items: center;
  gap: 32px;
  padding: 12px 4px;
}

.donut-wrap {
  position: relative;
  width: min(100%, 205px);
  aspect-ratio: 1;
  margin: 0 auto;
}

.donut {
  position: absolute;
  inset: 0;
  border-radius: 50%;
  box-shadow: 0 0 40px rgba(94, 234, 212, 0.06);
}

.donut::after {
  position: absolute;
  inset: 27px;
  border: 1px solid var(--border);
  border-radius: 50%;
  background: var(--surface);
  content: "";
}

.donut-center {
  position: absolute;
  inset: 0;
  z-index: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  pointer-events: none;
}

.donut-center strong {
  font-family: var(--font-mono);
  font-size: 1.25rem;
  font-weight: 600;
  letter-spacing: -0.04em;
}

.donut-center span {
  margin-top: 2px;
  color: var(--muted-2);
  font-size: 0.75rem;
  font-weight: 500;
}

.legend {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.legend-item {
  display: grid;
  grid-template-columns: 9px minmax(80px, 1fr) auto auto;
  align-items: center;
  gap: 8px;
  min-height: 27px;
  padding-bottom: 8px;
  border-bottom: 1px solid rgba(38, 50, 67, 0.55);
}

.legend-item:last-child {
  border-bottom: 0;
}

.legend-dot {
  width: 8px;
  height: 8px;
  border-radius: 3px;
}

.legend-label {
  overflow: hidden;
  color: var(--text-soft);
  font-size: 0.75rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.legend-value {
  color: var(--muted);
  font-family: var(--font-mono);
  font-size: 0.75rem;
  white-space: nowrap;
}

.legend-percent {
  min-width: 31px;
  color: var(--muted-2);
  font-family: var(--font-mono);
  font-size: 0.75rem;
  text-align: right;
}

.chart-empty {
  display: grid;
  min-height: 220px;
  place-items: center;
  border: 1px dashed var(--border-strong);
  border-radius: var(--radius);
  color: var(--muted-2);
  font-size: 0.8125rem;
}

@media (max-width: 740px) {
  .donut-layout {
    grid-template-columns: 1fr;
    gap: 20px;
  }

  .donut-wrap {
    width: min(100%, 180px);
  }
}
</style>
