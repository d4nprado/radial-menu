<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import {
  fallbackTextForItem,
  fixedIconForItem,
  resolveProgramIcon,
  type FixedIconKind,
  type ProgramIconPayload,
} from '../composables/useResolvedMenuIcon'
import type { MenuItem } from '../types/menu'

const props = defineProps<{
  item: MenuItem
  fixedIconOverride?: FixedIconKind | null
}>()

const programIcon = ref<ProgramIconPayload | null>(null)
const canvas = ref<HTMLCanvasElement | null>(null)
const fixedIcon = computed(() => props.fixedIconOverride ?? fixedIconForItem(props.item))
const fallbackText = computed(() => fallbackTextForItem(props.item))

watch(
  () => props.item.action.type === 'program' ? props.item.action.path : null,
  async (path) => {
    programIcon.value = null
    if (!path) return

    const requestedPath = path
    const resolved = await resolveProgramIcon(requestedPath)
    if (
      props.item.action.type === 'program'
      && props.item.action.path === requestedPath
    ) {
      programIcon.value = resolved
    }
  },
  { immediate: true },
)

watch(programIcon, async (icon) => {
  if (!icon) return
  await nextTick()

  const context = canvas.value?.getContext('2d')
  if (!context) return
  const image = new ImageData(
    new Uint8ClampedArray(icon.rgba),
    icon.width,
    icon.height,
  )
  context.putImageData(image, 0, 0)
})
</script>

<template>
  <canvas
    v-if="programIcon"
    ref="canvas"
    class="menu-item-icon menu-item-icon--canvas"
    :width="programIcon.width"
    :height="programIcon.height"
    aria-hidden="true"
  />

  <svg
    v-else-if="fixedIcon"
    class="menu-item-icon menu-item-icon--svg"
    viewBox="0 0 24 24"
    aria-hidden="true"
  >
    <template v-if="fixedIcon === 'folder'">
      <path d="M3 7.5h7l2-2h3.5a2 2 0 0 1 2 2v1H5a2 2 0 0 0-2 2z" />
      <path d="M3 10.5a2 2 0 0 1 2-2h15l-2.1 9a2 2 0 0 1-1.95 1.55H5a2 2 0 0 1-2-2z" />
    </template>
    <template v-else-if="fixedIcon === 'globe'">
      <circle cx="12" cy="12" r="8.5" />
      <path d="M3.8 12h16.4M12 3.5c2.2 2.3 3.3 5.1 3.3 8.5S14.2 18.2 12 20.5C9.8 18.2 8.7 15.4 8.7 12S9.8 5.8 12 3.5z" />
    </template>
    <template v-else-if="fixedIcon === 'terminal'">
      <rect x="3" y="4" width="18" height="16" rx="3" />
      <path d="m7 9 3 3-3 3M12.5 15H17" />
    </template>
    <template v-else-if="fixedIcon === 'calculator'">
      <rect x="5" y="2.5" width="14" height="19" rx="2.5" />
      <path d="M8 6h8v3H8zM8 13h.01M12 13h.01M16 13h.01M8 17h.01M12 17h.01M16 17h.01" />
    </template>
    <template v-else-if="fixedIcon === 'notepad'">
      <path d="M6 3.5h12a2 2 0 0 1 2 2v15H4v-15a2 2 0 0 1 2-2z" />
      <path d="M8 2v4M12 2v4M16 2v4M8 10h8M8 14h8M8 18h5" />
    </template>
    <template v-else-if="fixedIcon === 'broadcast'">
      <circle cx="12" cy="12" r="2.4" />
      <path d="M8.5 8.5a5 5 0 0 0 0 7M15.5 8.5a5 5 0 0 1 0 7" />
      <path d="M5.5 5.5a9.2 9.2 0 0 0 0 13M18.5 5.5a9.2 9.2 0 0 1 0 13" />
    </template>
    <template v-else-if="fixedIcon === 'mic' || fixedIcon === 'mic-muted'">
      <path d="M12 14.5a3 3 0 0 0 3-3v-5a3 3 0 0 0-6 0v5a3 3 0 0 0 3 3z" />
      <path d="M6.5 10.5v1a5.5 5.5 0 0 0 11 0v-1M12 17v3M9 20h6" />
      <path v-if="fixedIcon === 'mic-muted'" d="M4.5 4.5 19.5 19.5" />
    </template>
    <template v-else-if="fixedIcon === 'speaker' || fixedIcon === 'speaker-muted'">
      <path d="M4 9.5v5h4l5 4v-13l-5 4z" />
      <path v-if="fixedIcon === 'speaker'" d="M16 9a4.5 4.5 0 0 1 0 6M18.5 6.5a8 8 0 0 1 0 11" />
      <path v-else d="m16 10 5 5M21 10l-5 5" />
    </template>
    <template v-else-if="fixedIcon === 'audio' || fixedIcon === 'audio-muted'">
      <path d="M5 10v4h3l4 3.2V6.8L8 10z" />
      <path d="M16.5 8.5a5 5 0 0 1 0 7" />
      <path v-if="fixedIcon === 'audio-muted'" d="M4.5 4.5 19.5 19.5" />
    </template>
    <template v-else>
      <rect x="3" y="3" width="7" height="7" rx="1.5" />
      <rect x="14" y="3" width="7" height="7" rx="1.5" />
      <rect x="3" y="14" width="7" height="7" rx="1.5" />
      <rect x="14" y="14" width="7" height="7" rx="1.5" />
    </template>
  </svg>

  <span v-else class="menu-item-icon menu-item-icon--fallback">
    {{ fallbackText }}
  </span>
</template>

<style scoped>
.menu-item-icon {
  display: block;
  width: 68%;
  height: 68%;
  pointer-events: none;
  user-select: none;
}

.menu-item-icon--canvas {
  filter: drop-shadow(0 2px 4px rgb(0 0 0 / 35%));
}

.menu-item-icon--svg {
  overflow: visible;
  fill: none;
  stroke: currentcolor;
  stroke-width: 1.65;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.menu-item-icon--fallback {
  display: grid;
  width: 100%;
  height: 100%;
  place-items: center;
  font: inherit;
}
</style>
