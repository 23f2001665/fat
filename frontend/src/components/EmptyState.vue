<script setup>
import Icon from './Icon.vue'

defineProps({
  icon: { type: String, default: 'sparkles' },
  title: { type: String, default: 'Nothing here yet' },
  message: { type: String, default: '' },
})
</script>

<template>
  <div class="empty-panel">
    <span class="empty-icon">
      <Icon :name="icon" :size="23" />
    </span>
    <div>
      <h3>{{ title }}</h3>
      <p v-if="message" class="empty-message">{{ message }}</p>
    </div>
    <slot />
  </div>
</template>

<style scoped>
.empty-message {
  margin-top: 6px;
}
</style>
