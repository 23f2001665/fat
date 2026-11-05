<script setup>
import Icon from './Icon.vue'

defineProps({
  modelValue: { type: String, default: '' },
  placeholder: { type: String, default: 'Search…' },
  label: { type: String, default: 'Search' },
})

defineEmits(['update:modelValue'])
</script>

<template>
  <label class="search-field">
    <span class="visually-hidden">{{ label }}</span>
    <Icon name="search" :size="16" class="search-icon" />
    <input
      class="input search-input"
      type="search"
      :value="modelValue"
      :placeholder="placeholder"
      @input="$emit('update:modelValue', $event.target.value)"
    />
    <button
      v-if="modelValue"
      class="clear-button"
      type="button"
      aria-label="Clear search"
      @click="$emit('update:modelValue', '')"
    >
      <Icon name="close" :size="13" />
    </button>
  </label>
</template>

<style scoped>
.search-field {
  position: relative;
  display: block;
  min-width: 210px;
}

.search-icon {
  position: absolute;
  top: 50%;
  left: 12px;
  color: var(--muted-2);
  pointer-events: none;
  transform: translateY(-50%);
}

.search-input {
  height: 40px;
  min-height: 40px;
  padding-left: 36px;
  padding-right: 34px;
  font-size: 0.82rem;
}

.search-input::-webkit-search-cancel-button {
  display: none;
}

.clear-button {
  position: absolute;
  top: 50%;
  right: 7px;
  display: grid;
  width: 26px;
  height: 26px;
  padding: 0;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: var(--muted-2);
  place-items: center;
  transform: translateY(-50%);
}

.clear-button:hover {
  background: var(--surface-3);
  color: var(--text);
}
</style>
