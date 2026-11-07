<script setup>
import { computed } from 'vue'

const props = defineProps({
  name: { type: String, required: true },
  size: { type: [Number, String], default: 20 },
  title: { type: String, default: '' },
})

const icons = {
  dumbbell:
    '<path d="M6.5 6.5v11"/><path d="M3.5 9v6"/><path d="M17.5 6.5v11"/><path d="M20.5 9v6"/><path d="M6.5 12h11"/>',
  history:
    '<path d="M3 12a9 9 0 1 0 3-6.7L3 8"/><path d="M3 3v5h5"/><path d="M12 7v5l3 2"/>',
  analytics: '<path d="M3 3v18h18"/><path d="m7 15 4-4 3 3 5-6"/>',
  library:
    '<path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/>',
  profile: '<circle cx="12" cy="8" r="4"/><path d="M4 21a8 8 0 0 1 16 0"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  edit:
    '<path d="M12 20h9"/><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z"/>',
  trash:
    '<path d="M3 6h18"/><path d="M8 6V4h8v2"/><path d="M19 6l-1 14H6L5 6"/><path d="M10 11v6M14 11v6"/>',
  search: '<circle cx="11" cy="11" r="7"/><path d="m20 20-3-3"/>',
  chevronLeft: '<path d="m15 18-6-6 6-6"/>',
  chevronRight: '<path d="m9 18 6-6-6-6"/>',
  arrowUp: '<path d="m18 15-6-6-6 6"/>',
  arrowDown: '<path d="m6 9 6 6 6-6"/>',
  close: '<path d="M18 6 6 18M6 6l12 12"/>',
  calendar:
    '<rect x="3" y="4" width="18" height="18" rx="2"/><path d="M16 2v4M8 2v4M3 10h18"/>',
  filter: '<path d="M22 3H2l8 9.5V19l4 2v-8.5Z"/>',
  alert:
    '<path d="m21.7 18-8-14a2 2 0 0 0-3.4 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.7-3Z"/><path d="M12 9v4M12 17h.01"/>',
  check: '<path d="M20 6 9 17l-5-5"/>',
  refresh:
    '<path d="M20 11a8.1 8.1 0 0 0-15.5-2M4 4v5h5"/><path d="M4 13a8.1 8.1 0 0 0 15.5 2M20 20v-5h-5"/>',
  info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v5M12 8h.01"/>',
  flame:
    '<path d="M8.5 14.5A2.5 2.5 0 0 0 11 17c1.4 0 2.5-1.1 2.5-2.5 0-1.7-1.6-2.8-1.6-4.5 0 0 3.1 1.3 3.1 4.5a5 5 0 1 1-10 0c0-2.2 1.5-4.2 1.5-4.2s.8 1.8 2 2.2c0-2.8 1.5-5.5 4-7 0 2.2 2.5 4 2.5 7"/>',
  target:
    '<circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="5"/><circle cx="12" cy="12" r="1"/>',
  scale:
    '<path d="M12 3v3"/><path d="M5 21h14a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v11a2 2 0 0 0 2 2Z"/><path d="M8 3h8"/><path d="M12 12a2 2 0 1 0 0-4 2 2 0 0 0 0 4Z"/><path d="m12 12 3 4"/>',
  ruler:
    '<path d="m16 2 6 6L8 22l-6-6Z"/><path d="m7 15 2 2M10 12l2 2M13 9l2 2M16 6l2 2"/>',
  clock: '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
  sparkles:
    '<path d="m12 3-1.2 3.8a2 2 0 0 1-1.3 1.3L5.7 9.3l3.8 1.2a2 2 0 0 1 1.3 1.3L12 15.6l1.2-3.8a2 2 0 0 1 1.3-1.3l3.8-1.2-3.8-1.2a2 2 0 0 1-1.3-1.3Z"/><path d="M5 17v4M3 19h4"/>',
  shield:
    '<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10Z"/><path d="m9 12 2 2 4-4"/>',
  database:
    '<ellipse cx="12" cy="5" rx="8" ry="3"/><path d="M4 5v6c0 1.7 3.6 3 8 3s8-1.3 8-3V5"/><path d="M4 11v6c0 1.7 3.6 3 8 3s8-1.3 8-3v-6"/>',
  chartBar: '<path d="M3 21V9M9 21V3M15 21v-7M21 21V6"/>',
  external: '<path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>',
  circle: '<circle cx="12" cy="12" r="9"/>',
  checkCircle: '<circle cx="12" cy="12" r="9"/><path d="m8.5 12 2.4 2.4 4.8-5"/>',
  users:
    '<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/>',
  userPlus:
    '<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M19 8v6M22 11h-6"/>',
  logout:
    '<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><path d="m16 17 5-5-5-5"/><path d="M21 12H9"/>',
  key:
    '<circle cx="7.5" cy="15.5" r="4.5"/><path d="m10.7 12.3 8.3-8.3"/><path d="m17 6 3 3"/><path d="m15 8 3 3"/>',
}

const markup = computed(() => icons[props.name] || icons.circle)
</script>

<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.8"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <title v-if="title">{{ title }}</title>
    <g v-html="markup" />
  </svg>
</template>
