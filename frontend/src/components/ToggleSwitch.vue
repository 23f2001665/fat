<script setup>
defineProps({
  modelValue: { type: Boolean, default: false },
  label: { type: String, default: '' },
  disabled: { type: Boolean, default: false },
})

defineEmits(['update:modelValue'])
</script>

<template>
  <button
    type="button"
    class="switch"
    role="switch"
    :aria-checked="modelValue"
    :disabled="disabled"
    @click="$emit('update:modelValue', !modelValue)"
  >
    <span class="switch-track" aria-hidden="true">
      <span class="switch-dot" />
    </span>
    <span v-if="label">{{ label }}</span>
  </button>
</template>

<style scoped>
.switch:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}
</style>
