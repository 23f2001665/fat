<script setup>
import Icon from './Icon.vue'

defineProps({
  label: { type: String, required: true },
  value: { type: [String, Number], default: '—' },
  unit: { type: String, default: '' },
  hint: { type: String, default: '' },
  icon: { type: String, default: 'sparkles' },
  tone: { type: String, default: 'accent' },
  compact: { type: Boolean, default: false },
})
</script>

<template>
  <div class="stat-card card" :class="[`tone-${tone}`, { compact }]">
    <div class="stat-top">
      <span class="stat-label">{{ label }}</span>
      <span class="stat-icon">
        <Icon :name="icon" :size="17" />
      </span>
    </div>
    <div class="stat-main">
      <span class="stat-value">{{ value }}</span>
      <span v-if="unit" class="stat-unit">{{ unit }}</span>
    </div>
    <p v-if="hint" class="stat-hint">{{ hint }}</p>
  </div>
</template>

<style scoped>
.stat-card {
  position: relative;
  min-height: 116px;
  padding: 16px 17px;
  overflow: hidden;
}

.stat-card::after {
  position: absolute;
  top: -30px;
  right: -30px;
  width: 110px;
  height: 110px;
  border-radius: 50%;
  background: var(--tone-soft);
  content: "";
  filter: blur(2px);
  opacity: 0.6;
  pointer-events: none;
}

.tone-accent {
  --tone: var(--accent);
  --tone-soft: var(--accent-soft);
}

.tone-info {
  --tone: var(--info);
  --tone-soft: var(--info-soft);
}

.tone-warning {
  --tone: var(--warning);
  --tone-soft: var(--warning-soft);
}

.tone-danger {
  --tone: var(--danger);
  --tone-soft: var(--danger-soft);
}

.tone-violet {
  --tone: var(--violet);
  --tone-soft: rgba(196, 181, 253, 0.1);
}

.stat-top {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.stat-label {
  color: var(--muted);
  font-size: 0.71rem;
  font-weight: 650;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.stat-icon {
  display: grid;
  width: 30px;
  height: 30px;
  place-items: center;
  border: 1px solid color-mix(in srgb, var(--tone) 28%, transparent);
  border-radius: 9px;
  background: var(--tone-soft);
  color: var(--tone);
}

.stat-main {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: baseline;
  gap: 5px;
  margin-top: 11px;
}

.stat-value {
  color: var(--text);
  font-size: clamp(1.45rem, 2.4vw, 1.85rem);
  font-weight: 600;
  line-height: 1;
}

.stat-unit {
  color: var(--muted);
  font-size: 0.72rem;
  font-weight: 550;
}

.stat-hint {
  position: relative;
  z-index: 1;
  margin-top: 9px;
  color: var(--muted-2);
  font-size: 0.68rem;
  line-height: 1.35;
}

.stat-card.compact {
  min-height: 86px;
  padding: 13px 14px;
}

.stat-card.compact .stat-main {
  margin-top: 7px;
}

.stat-card.compact .stat-value {
  font-size: 1.25rem;
}
</style>
