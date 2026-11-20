<script setup>
import { computed } from 'vue'
import { formatDateShort, formatNumber, parseISODate } from '@/utils/format'

const props = defineProps({
  points: { type: Array, default: () => [] },
  unit: { type: String, default: '' },
  height: { type: Number, default: 260 },
  maxDecimals: { type: Number, default: 1 },
  color: { type: String, default: '#5eead4' },
  label: { type: String, default: 'Progress chart' },
})

let chartCounter = 0
const gradientId = `line-chart-fill-${++chartCounter}`

const width = 820
const margins = { top: 18, right: 22, bottom: 38, left: 58 }
const plotWidth = width - margins.left - margins.right
const plotHeight = props.height - margins.top - margins.bottom

const rows = computed(() =>
  props.points
    .map((point) => ({
      date: point.date,
      value: Number(point.value),
      timestamp: parseISODate(point.date)?.getTime() ?? null,
    }))
    .filter((point) => Number.isFinite(point.value)),
)

const domain = computed(() => {
  if (!rows.value.length) return { min: 0, max: 1, ticks: [] }
  const values = rows.value.map((row) => row.value)
  let min = Math.min(...values)
  let max = Math.max(...values)
  if (max === min) {
    const padding = Math.abs(max) * 0.12 || 1
    min -= padding
    max += padding
  } else {
    const padding = (max - min) * 0.16
    min -= padding
    max += padding
  }
  min = Math.max(0, min)
  const ticks = Array.from({ length: 4 }, (_, index) => min + ((max - min) * index) / 3)
  return { min, max, ticks }
})

function xFor(row, index) {
  if (rows.value.length === 1) return margins.left + plotWidth / 2
  const timestamps = rows.value.map((item) => item.timestamp).filter((value) => value !== null)
  if (timestamps.length === rows.value.length) {
    const minTime = Math.min(...timestamps)
    const maxTime = Math.max(...timestamps)
    if (maxTime === minTime) return margins.left + plotWidth / 2
    return margins.left + ((row.timestamp - minTime) / (maxTime - minTime)) * plotWidth
  }
  return margins.left + (index / (rows.value.length - 1)) * plotWidth
}

function yFor(value) {
  const { min, max } = domain.value
  if (max === min) return margins.top + plotHeight / 2
  return margins.top + plotHeight - ((value - min) / (max - min)) * plotHeight
}

const plotted = computed(() =>
  rows.value.map((row, index) => ({
    ...row,
    x: xFor(row, index),
    y: yFor(row.value),
  })),
)

const linePath = computed(() =>
  plotted.value.map((point, index) => `${index === 0 ? 'M' : 'L'} ${point.x} ${point.y}`).join(' '),
)

const areaPath = computed(() => {
  if (!plotted.value.length) return ''
  const baseline = margins.top + plotHeight
  const first = plotted.value[0]
  const last = plotted.value[plotted.value.length - 1]
  return `${linePath.value} L ${last.x} ${baseline} L ${first.x} ${baseline} Z`
})

function formatAxis(value) {
  if (Math.abs(value) >= 10000) return `${formatNumber(value / 1000, 0)}k`
  return formatNumber(value, props.maxDecimals)
}

function pointTooltip(point) {
  return `${formatDateShort(point.date)} · ${formatNumber(point.value, 2)}${props.unit ? ` ${props.unit}` : ''}`
}

const xLabels = computed(() => {
  if (!plotted.value.length) return []
  if (plotted.value.length === 1) {
    return [{ ...plotted.value[0], anchor: 'middle' }]
  }
  const first = plotted.value[0]
  const last = plotted.value[plotted.value.length - 1]
  return [
    { ...first, anchor: 'start' },
    { ...last, anchor: 'end' },
  ]
})
</script>

<template>
  <div v-if="!plotted.length" class="chart-empty">
    <span>No recorded points in this range.</span>
  </div>
  <div v-else class="line-chart">
    <svg
      :viewBox="`0 0 ${width} ${height}`"
      role="img"
      :aria-label="label"
      preserveAspectRatio="xMidYMid meet"
    >
      <defs>
        <linearGradient :id="gradientId" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" :stop-color="color" stop-opacity="0.24" />
          <stop offset="100%" :stop-color="color" stop-opacity="0" />
        </linearGradient>
      </defs>

      <g class="grid">
        <line
          v-for="tick in domain.ticks"
          :key="tick"
          :x1="margins.left"
          :x2="width - margins.right"
          :y1="yFor(tick)"
          :y2="yFor(tick)"
        />
      </g>

      <g class="axis-labels">
        <text
          v-for="tick in domain.ticks"
          :key="tick"
          :x="margins.left - 12"
          :y="yFor(tick) + 4"
          text-anchor="end"
        >
          {{ formatAxis(tick) }}
        </text>
      </g>

      <path :d="areaPath" :fill="`url(#${gradientId})`" />
      <path
        :d="linePath"
        fill="none"
        :stroke="color"
        stroke-width="2.6"
        stroke-linecap="round"
        stroke-linejoin="round"
      />

      <g class="points">
        <circle
          v-for="point in plotted"
          :key="point.date"
          :cx="point.x"
          :cy="point.y"
          r="4.2"
          :fill="color"
          stroke="#0b0f14"
          stroke-width="2.4"
        >
          <title>{{ pointTooltip(point) }}</title>
        </circle>
      </g>

      <g class="x-labels">
        <text
          v-for="point in xLabels"
          :key="`x-${point.date}-${point.anchor}`"
          :x="point.x"
          :y="height - 12"
          :text-anchor="point.anchor"
        >
          {{ formatDateShort(point.date) }}
        </text>
      </g>
    </svg>
  </div>
</template>

<style scoped>
.line-chart {
  width: 100%;
}

svg {
  display: block;
  width: 100%;
  height: auto;
  overflow: visible;
}

.grid line {
  stroke: #253142;
  stroke-dasharray: 3 6;
  stroke-width: 1;
}

.axis-labels text,
.x-labels text {
  fill: var(--muted-2);
  font-family: var(--font-mono);
  font-size: 10px;
}

.points circle {
  cursor: help;
  transition: r 120ms ease;
}

.points circle:hover {
  r: 6;
}

.chart-empty {
  display: grid;
  min-height: 190px;
  place-items: center;
  border: 1px dashed var(--border-strong);
  border-radius: var(--radius);
  color: var(--muted-2);
  font-size: 0.8125rem;
}
</style>
