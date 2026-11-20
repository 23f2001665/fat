<script setup>
import { computed } from 'vue'
import Icon from './Icon.vue'
import { WEEKLY_GOAL_OPTIONS, goalState, setWeeklyGoal } from '@/stores/goals'
import { summariseWeek } from '@/utils/weeks'

const props = defineProps({
  trainedDates: { type: Array, default: () => [] },
  editing: { type: Boolean, default: false },
})

const emit = defineEmits(['update:editing'])

const week = computed(() => summariseWeek(props.trainedDates, goalState.weeklyGoal))

const segments = computed(() =>
  Array.from({ length: week.value.goal }, (_, index) => index < week.value.sessions),
)

const message = computed(() => {
  const { met, remaining, daysLeft, previousStreak, streak } = week.value
  const sessions = remaining === 1 ? 'session' : 'sessions'
  if (met) {
    return streak > 1 ? `Goal reached. Your ${streak}-week streak is safe.` : 'Goal reached for this week.'
  }
  if (remaining > daysLeft) {
    return previousStreak
      ? `Not enough days left to keep the ${previousStreak}-week streak. Start fresh on Monday.`
      : 'Not enough days left this week. Start fresh on Monday.'
  }
  if (previousStreak) {
    return `${remaining} more ${sessions} by Sunday to keep your ${previousStreak}-week streak.`
  }
  return `${remaining} more ${sessions} to reach this week’s goal.`
})

const atRisk = computed(
  () => !week.value.met && week.value.previousStreak > 0,
)

function choose(value) {
  setWeeklyGoal(value)
  emit('update:editing', false)
}
</script>

<template>
  <section class="goal-card card" aria-label="Weekly goal">
    <div class="goal-main">
      <div class="goal-head">
        <h2>This week</h2>
        <p class="goal-count">
          <strong class="mono">{{ week.sessions }}</strong>
          <span>of {{ week.goal }} sessions</span>
        </p>
      </div>

      <div class="goal-segments" role="progressbar" :aria-valuenow="week.sessions" :aria-valuemax="week.goal" aria-valuemin="0">
        <span v-for="(filled, index) in segments" :key="index" class="segment-bar" :class="{ filled }" />
      </div>

      <ol class="week-days">
        <li
          v-for="day in week.days"
          :key="day.iso"
          :class="{ trained: day.trained, today: day.isToday, future: day.isFuture }"
          :title="day.iso"
        >
          <span class="day-dot">
            <Icon v-if="day.trained" name="check" :size="12" />
          </span>
          <span class="day-letter">{{ day.letter }}</span>
        </li>
      </ol>
    </div>

    <div class="goal-side">
      <span class="streak" :class="{ live: week.streak > 0, risk: atRisk }">
        <Icon name="flame" :size="16" />
        <template v-if="week.streak">{{ week.streak }}-week streak</template>
        <template v-else>No streak yet</template>
      </span>
      <p class="goal-message" :class="{ risk: atRisk }">{{ message }}</p>
      <button
        class="btn btn-ghost btn-sm goal-edit"
        type="button"
        :aria-expanded="editing"
        @click="emit('update:editing', !editing)"
      >
        <Icon name="target" :size="14" />
        {{ goalState.goalChosen ? 'Change goal' : 'Set your goal' }}
      </button>
    </div>

    <div v-if="editing" class="goal-editor">
      <p class="goal-editor-title">How many sessions a week do you want to train?</p>
      <div class="goal-options" role="radiogroup" aria-label="Weekly session goal">
        <button
          v-for="option in WEEKLY_GOAL_OPTIONS"
          :key="option.value"
          class="goal-option"
          :class="{ active: goalState.weeklyGoal === option.value, recommended: option.note === 'Recommended' }"
          type="button"
          role="radio"
          :aria-checked="goalState.weeklyGoal === option.value"
          @click="choose(option.value)"
        >
          <strong>{{ option.value }}</strong>
          <span>{{ option.note || 'per week' }}</span>
        </button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.goal-card {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 300px);
  gap: 24px;
  padding: 20px;
}

.goal-main {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 12px;
}

.goal-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
}

.goal-head h2 {
  font-size: var(--text-lg);
}

.goal-count {
  display: flex;
  align-items: baseline;
  gap: 6px;
  color: var(--muted);
  font-size: var(--text-sm);
}

.goal-count strong {
  color: var(--text);
  font-size: 1.25rem;
  font-weight: 600;
}

.goal-segments {
  display: flex;
  gap: 4px;
}

.segment-bar {
  height: 6px;
  flex: 1;
  border-radius: 999px;
  background: var(--surface-3);
  transition: background 300ms ease;
}

.segment-bar.filled {
  background: var(--accent);
}

.week-days {
  display: grid;
  grid-template-columns: repeat(7, minmax(0, 1fr));
  gap: 4px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.week-days li {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.day-dot {
  display: grid;
  width: 24px;
  height: 24px;
  place-items: center;
  border: 1px solid var(--border-strong);
  border-radius: 50%;
  color: var(--accent-ink);
}

.week-days li.future .day-dot {
  border-style: dashed;
  border-color: var(--border);
}

.week-days li.today .day-dot {
  border-color: var(--accent);
}

.week-days li.trained .day-dot {
  border-color: var(--accent);
  background: var(--accent);
}

.day-letter {
  color: var(--muted);
  font-size: var(--text-xs);
  font-weight: 500;
}

.week-days li.today .day-letter {
  color: var(--accent);
  font-weight: 600;
}

.goal-side {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  justify-content: center;
  gap: 8px;
  padding-left: 24px;
  border-left: 1px solid var(--divider);
}

.streak {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--muted);
  font-size: var(--text-base);
  font-weight: 600;
}

.streak.live {
  color: var(--warning);
}

.goal-message {
  color: var(--text-soft);
  font-size: var(--text-sm);
  line-height: 1.5;
}

.goal-message.risk {
  color: var(--warning);
}

.goal-edit {
  margin-left: -10px;
}

.goal-editor {
  grid-column: 1 / -1;
  padding-top: 16px;
  border-top: 1px solid var(--divider);
}

.goal-editor-title {
  margin-bottom: 12px;
  color: var(--text-soft);
  font-size: var(--text-sm);
  font-weight: 500;
}

.goal-options {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 8px;
}

.goal-option {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 8px 4px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface-2);
  color: var(--text-soft);
  transition: border-color 150ms ease, background 150ms ease;
}

.goal-option strong {
  font-family: var(--font-mono);
  font-size: var(--text-lg);
  font-weight: 600;
}

.goal-option span {
  color: var(--muted);
  font-size: var(--text-xs);
}

.goal-option.recommended span {
  color: var(--accent);
}

.goal-option:hover {
  border-color: var(--border-strong);
}

.goal-option.active {
  border-color: var(--accent);
  background: var(--accent-soft);
}

@media (max-width: 760px) {
  .goal-card {
    grid-template-columns: 1fr;
    gap: 16px;
    padding: 16px;
  }

  .goal-side {
    padding-top: 16px;
    padding-left: 0;
    border-top: 1px solid var(--divider);
    border-left: 0;
  }
}

@media (max-width: 420px) {
  .goal-options {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
}
</style>
