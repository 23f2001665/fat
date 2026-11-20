<script setup>
import { computed } from 'vue'
import Icon from './Icon.vue'

const props = defineProps({
  hasSets: { type: Boolean, default: false },
  hasMeasurement: { type: Boolean, default: false },
  goalChosen: { type: Boolean, default: false },
})

const emit = defineEmits(['log-set', 'choose-goal', 'dismiss'])

// The first step is already complete when the card first appears: a head start
// makes the remaining steps feel closer (goal-gradient effect).
const steps = computed(() => [
  { key: 'account', label: 'Create your account', done: true },
  { key: 'goal', label: 'Choose a weekly goal', done: props.goalChosen, action: 'Choose' },
  { key: 'set', label: 'Log your first set', done: props.hasSets, action: 'Log a set' },
  { key: 'measure', label: 'Add a body measurement', done: props.hasMeasurement, action: 'Add', to: '/profile?add=measurement' },
])

const doneCount = computed(() => steps.value.filter((step) => step.done).length)
const percent = computed(() => Math.round((doneCount.value / steps.value.length) * 100))

function run(step) {
  if (step.key === 'goal') emit('choose-goal')
  if (step.key === 'set') emit('log-set')
}
</script>

<template>
  <section class="start-card card" aria-label="Getting started">
    <header class="start-header">
      <div>
        <h2>Getting started</h2>
        <p>{{ doneCount }} of {{ steps.length }} done</p>
      </div>
      <button
        class="btn btn-ghost btn-icon btn-sm"
        type="button"
        aria-label="Hide getting started"
        title="Hide"
        @click="emit('dismiss')"
      >
        <Icon name="close" :size="14" />
      </button>
    </header>

    <div class="progress" role="progressbar" :aria-valuenow="percent" aria-valuemin="0" aria-valuemax="100">
      <div class="progress-fill" :style="{ width: `${percent}%` }" />
    </div>

    <ol class="steps">
      <li v-for="step in steps" :key="step.key" :class="{ done: step.done }">
        <span class="step-mark">
          <Icon v-if="step.done" name="check" :size="12" />
        </span>
        <span class="step-label">{{ step.label }}</span>
        <template v-if="!step.done">
          <RouterLink v-if="step.to" :to="step.to" class="btn btn-secondary btn-sm">
            {{ step.action }}
          </RouterLink>
          <button v-else class="btn btn-secondary btn-sm" type="button" @click="run(step)">
            {{ step.action }}
          </button>
        </template>
      </li>
    </ol>
  </section>
</template>

<style scoped>
.start-card {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 20px;
}

.start-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.start-header h2 {
  font-size: var(--text-lg);
}

.start-header p {
  margin-top: 2px;
  color: var(--muted);
  font-size: var(--text-sm);
}

.steps {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
  margin: 4px 0 0;
  padding: 0;
  list-style: none;
}

.steps li {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  min-height: 48px;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface-2);
}

.steps li.done {
  border-color: transparent;
  background: transparent;
}

.step-mark {
  display: grid;
  width: 20px;
  height: 20px;
  flex: 0 0 auto;
  place-items: center;
  border: 1px solid var(--border-strong);
  border-radius: 50%;
  color: var(--accent-ink);
}

.steps li.done .step-mark {
  border-color: var(--accent);
  background: var(--accent);
}

.step-label {
  min-width: 0;
  flex: 1;
  color: var(--text-soft);
  font-size: var(--text-sm);
  font-weight: 500;
}

.steps li.done .step-label {
  color: var(--muted);
  text-decoration: line-through;
  text-decoration-color: var(--border-strong);
}

@media (max-width: 1100px) {
  .steps {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 560px) {
  .start-card {
    padding: 16px;
  }

  .steps {
    grid-template-columns: 1fr;
  }
}
</style>
