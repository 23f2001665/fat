<script setup>
defineProps({
  eyebrow: { type: String, default: '' },
  title: { type: String, required: true },
  description: { type: String, default: '' },
})
</script>

<template>
  <header class="page-header">
    <div class="page-heading">
      <p v-if="eyebrow" class="eyebrow">{{ eyebrow }}</p>
      <h1>{{ title }}</h1>
      <p v-if="description" class="page-description">{{ description }}</p>
    </div>
    <div v-if="$slots.actions" class="page-actions">
      <slot name="actions" />
    </div>
  </header>
</template>

<style scoped>
.page-header {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 20px;
  padding-top: 4px;
}

.page-heading {
  min-width: 0;
}

.page-description {
  max-width: 700px;
  margin-top: 8px;
  color: var(--muted);
  font-size: 0.9375rem;
  line-height: 1.6;
}

.page-actions {
  display: flex;
  flex: 0 0 auto;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}

@media (max-width: 760px) {
  .page-header {
    flex-direction: column;
    align-items: stretch;
    gap: 16px;
  }

  .page-actions {
    width: 100%;
  }

  .page-actions :deep(.btn) {
    flex: 1 1 auto;
    white-space: nowrap;
  }
}
</style>
