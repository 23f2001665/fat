<script setup>
import { computed } from 'vue'
import { formatCompact, formatNumber } from '@/utils/format'

const props = defineProps({
  items: { type: Array, default: () => [] },
  unit: { type: String, default: '' },
  horizontal: { type: Boolean, default: false },
  maxDecimals: { type: Number, default: 1 },
  color: { type: String, default: '#5eead4' },
  label: { type: String, default: 'Bar chart' },
})

const rows = computed(() =>
  props.items
    .map((item) => ({
      label: item.label,
      value: Number(item.value),
    }))
    .filter((item) => Number.isFinite(item.value) && item.value > 0),
)

const maxValue = computed(() => Math.max(...rows.value.map((row) => row.value), 0))

function widthFor(value) {
  if (!maxValue.value) return 0
  return Math.max(1.5, (value / maxValue.value) * 100)
}

function valueLabel(value) {
  return `${formatNumber(value, props.maxDecimals)}${props.unit ? ` ${props.unit}` : ''}`
}

function valueCompact(value) {
  return `${formatCompact(value)}${props.unit ? ` ${props.unit}` : ''}`
}
</script>

<template>
  <div v-if="!rows.length" class="chart-empty">
    <span>No recorded values in this range.</span>
  </div>

  <div v-else-if="horizontal" class="horizontal-chart" role="img" :aria-label="label">
    <div v-for="row in rows" :key="row.label" class="horizontal-row" :title="`${row.label}: ${valueLabel(row.value)}`">
      <span class="row-label">{{ row.label }}</span>
      <div class="row-track">
        <div class="row-fill" :style="{ width: `${widthFor(row.value)}%`, background: color }" />
      </div>
      <span class="row-value">{{ valueLabel(row.value) }}</span>
    </div>
  </div>

  <div v-else class="vertical-chart" role="img" :aria-label="label">
    <div class="vertical-scroll">
      <div class="vertical-grid">
        <span class="grid-line top" :data-value="valueCompact(maxValue)" />
        <span class="grid-line mid" :data-value="valueCompact(maxValue / 2)" />
        <span class="grid-line bottom" data-value="0" />
      </div>
      <div class="bars" :class="{ dense: rows.length > 10 }">
        <div
          v-for="row in rows"
          :key="row.label"
          class="bar-column"
          :title="`${row.label}: ${valueLabel(row.value)}`"
        >
          <span v-if="rows.length <= 12" class="bar-value">{{ valueCompact(row.value) }}</span>
          <div class="bar-track">
            <div
              class="bar-fill"
              :style="{
                height: `${widthFor(row.value)}%`,
                background: `linear-gradient(180deg, ${color}, color-mix(in srgb, ${color} 28%, transparent))`,
              }"
            />
          </div>
          <span class="bar-label">{{ row.label }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.vertical-scroll {
  position: relative;
  min-width: 100%;
  padding: 8px 0 0 43px;
  overflow-x: auto;
}

.vertical-chart {
  overflow: hidden;
}

.vertical-grid {
  position: absolute;
  top: 8px;
  right: 0;
  bottom: 43px;
  left: 0;
  pointer-events: none;
}

.grid-line {
  position: absolute;
  left: 0;
  width: 100%;
  border-top: 1px dashed #253142;
  color: var(--muted-2);
  font-family: var(--font-mono);
  font-size: 9px;
  line-height: 1;
}

.grid-line::before {
  position: absolute;
  top: -5px;
  left: 0;
  padding-right: 8px;
  background: var(--surface);
  content: attr(data-value);
}

.grid-line.top {
  top: 12px;
}

.grid-line.mid {
  top: 48%;
}

.grid-line.bottom {
  top: calc(100% - 1px);
}

.bars {
  display: flex;
  min-width: 100%;
  height: 235px;
  align-items: flex-end;
  gap: 14px;
}

.bars.dense {
  gap: 8px;
}

.bar-column {
  display: flex;
  min-width: 42px;
  flex: 1 0 42px;
  flex-direction: column;
  align-items: center;
  align-self: stretch;
  justify-content: flex-end;
  gap: 7px;
}

.bar-value {
  color: var(--muted);
  font-family: var(--font-mono);
  font-size: 9px;
  white-space: nowrap;
}

.bar-track {
  position: relative;
  display: flex;
  width: min(100%, 44px);
  height: 170px;
  align-items: flex-end;
}

.bar-fill {
  width: 100%;
  min-height: 3px;
  border-radius: 7px 7px 3px 3px;
  transform-origin: bottom;
  animation: bar-rise 480ms cubic-bezier(0.22, 1, 0.36, 1);
}

.bar-label {
  overflow: hidden;
  width: 100%;
  color: var(--muted-2);
  font-family: var(--font-mono);
  font-size: 9px;
  text-align: center;
  text-overflow: ellipsis;
  white-space: nowrap;
}

@keyframes bar-rise {
  from {
    transform: scaleY(0);
  }
}

.horizontal-chart {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 4px 0;
}

.horizontal-row {
  display: grid;
  grid-template-columns: minmax(100px, 1.2fr) minmax(120px, 2.2fr) auto;
  align-items: center;
  gap: 13px;
}

.row-label {
  overflow: hidden;
  color: var(--text-soft);
  font-size: 0.78rem;
  font-weight: 550;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.row-track {
  height: 9px;
  overflow: hidden;
  border-radius: 999px;
  background: var(--surface-3);
}

.row-fill {
  height: 100%;
  min-width: 3px;
  border-radius: inherit;
  opacity: 0.82;
  transform-origin: left;
  animation: row-grow 480ms cubic-bezier(0.22, 1, 0.36, 1);
}

.row-value {
  color: var(--muted);
  font-family: var(--font-mono);
  font-size: 0.7rem;
  text-align: right;
  white-space: nowrap;
}

@keyframes row-grow {
  from {
    transform: scaleX(0);
  }
}

.chart-empty {
  display: grid;
  min-height: 190px;
  place-items: center;
  border: 1px dashed var(--border-strong);
  border-radius: var(--radius);
  color: var(--muted-2);
  font-size: 0.8rem;
}

@media (max-width: 620px) {
  .horizontal-row {
    grid-template-columns: minmax(80px, 1fr) minmax(80px, 1.4fr) auto;
    gap: 9px;
  }

  .vertical-scroll {
    padding-left: 32px;
  }
}
</style>
