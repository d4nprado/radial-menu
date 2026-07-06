<script setup lang="ts">
import { computed, ref } from 'vue'
import type { MenuItem } from '../types/menu'
import MenuItemIcon from './MenuItemIcon.vue'

const props = defineProps<{
  items: MenuItem[]
  selectedId: string | null
  maxItems: number
  groupLabel: string | null
}>()

const emit = defineEmits<{
  select: [id: string]
  add: []
  reorder: [fromIndex: number, toIndex: number]
  openGroup: [id: string]
  back: []
}>()

const ring = ref<HTMLElement | null>(null)
const draggingId = ref<string | null>(null)
const dragTargetIndex = ref<number | null>(null)
const dragStart = ref({ x: 0, y: 0 })
const didDrag = ref(false)

const selectedItem = computed(() =>
  props.items.find((item) => item.id === props.selectedId),
)

function itemPosition(index: number) {
  const positionIndex = props.items[index]?.id === draggingId.value
    ? (dragTargetIndex.value ?? index)
    : index
  const angle = (positionIndex / props.items.length) * Math.PI * 2 - Math.PI / 2
  const radius = 150

  return {
    '--preview-x': `${Math.cos(angle) * radius}px`,
    '--preview-y': `${Math.sin(angle) * radius}px`,
    '--preview-accent': props.items[index]?.accent ?? '#8b7cff',
  }
}

function indexFromPointer(event: PointerEvent) {
  if (!ring.value || props.items.length < 2) return 0

  const rect = ring.value.getBoundingClientRect()
  const x = event.clientX - (rect.left + rect.width / 2)
  const y = event.clientY - (rect.top + rect.height / 2)
  const angleFromTop = (
    Math.atan2(y, x)
    + Math.PI / 2
    + Math.PI * 2
  ) % (Math.PI * 2)

  return Math.round(
    angleFromTop / (Math.PI * 2) * props.items.length,
  ) % props.items.length
}

function startDrag(event: PointerEvent, item: MenuItem, index: number) {
  if (event.button !== 0) return

  draggingId.value = item.id
  dragTargetIndex.value = index
  dragStart.value = { x: event.clientX, y: event.clientY }
  didDrag.value = false
  emit('select', item.id)

  if (event.currentTarget instanceof HTMLElement) {
    event.currentTarget.setPointerCapture(event.pointerId)
  }
}

function moveDrag(event: PointerEvent) {
  if (!draggingId.value) return

  const distance = Math.hypot(
    event.clientX - dragStart.value.x,
    event.clientY - dragStart.value.y,
  )
  if (distance < 5 && !didDrag.value) return

  didDrag.value = true
  dragTargetIndex.value = indexFromPointer(event)
}

function finishDrag(event: PointerEvent) {
  if (!draggingId.value) return

  const fromIndex = props.items.findIndex((item) => item.id === draggingId.value)
  const toIndex = dragTargetIndex.value ?? fromIndex
  if (didDrag.value && fromIndex >= 0 && toIndex !== fromIndex) {
    emit('reorder', fromIndex, toIndex)
  }

  if (
    event.currentTarget instanceof HTMLElement
    && event.currentTarget.hasPointerCapture(event.pointerId)
  ) {
    event.currentTarget.releasePointerCapture(event.pointerId)
  }
  draggingId.value = null
  dragTargetIndex.value = null
  requestAnimationFrame(() => {
    didDrag.value = false
  })
}

function activateItem(item: MenuItem) {
  if (didDrag.value) return
  if (item.action.type === 'group') {
    emit('openGroup', item.id)
  } else {
    emit('select', item.id)
  }
}
</script>

<template>
  <section class="preview-card" aria-labelledby="preview-title">
    <div class="preview-card__heading">
      <div>
        <span>VISUALIZAÇÃO</span>
        <h2 id="preview-title">Preview radial</h2>
      </div>
      <small>{{ items.length }} {{ items.length === 1 ? 'item' : 'itens' }}</small>
    </div>

    <div ref="ring" class="preview-ring">
      <div class="preview-ring__orbit preview-ring__orbit--outer" />
      <div class="preview-ring__orbit preview-ring__orbit--inner" />

      <button
        v-for="(item, index) in items"
        :key="item.id"
        type="button"
        class="preview-item"
        :class="{
          'is-selected': item.id === selectedId,
          'is-dragging': item.id === draggingId,
        }"
        :style="itemPosition(index)"
        :aria-label="`Selecionar ${item.label}`"
        :title="`${item.label} — ${item.hint}`"
        @click="activateItem(item)"
        @pointerdown="startDrag($event, item, index)"
        @pointermove="moveDrag"
        @pointerup="finishDrag"
        @pointercancel="finishDrag"
      >
        <span>
          <MenuItemIcon :item="item" />
        </span>
      </button>

      <button
        type="button"
        class="preview-center"
        :class="{ 'is-back': groupLabel }"
        :disabled="!groupLabel"
        :aria-label="groupLabel ? 'Voltar ao menu principal' : undefined"
        @click="groupLabel && emit('back')"
      >
        <strong>{{ groupLabel ?? 'ORBIT' }}</strong>
        <span>
          {{ groupLabel
            ? '← Voltar'
            : (selectedItem?.label ?? (items.length ? 'Selecione um item' : 'Nenhum item configurado')) }}
        </span>
      </button>
    </div>

    <div class="preview-card__selection">
      <div>
        <span class="preview-card__dot" :style="{ background: selectedItem?.accent }" />
        <strong>{{ selectedItem?.label ?? (items.length ? 'Nenhum item selecionado' : 'Menu vazio') }}</strong>
        <small v-if="selectedItem">{{ selectedItem.hint }}</small>
      </div>
      <button
        type="button"
        class="preview-card__add"
        :disabled="items.length >= maxItems"
        :title="items.length >= maxItems ? 'Limite de 10 itens neste nível' : 'Adicionar item'"
        @click="emit('add')"
      >
        <span aria-hidden="true">+</span>
        Adicionar ação
      </button>
    </div>
  </section>
</template>

<style scoped>
.preview-card {
  display: flex;
  min-height: 0;
  padding: 24px;
  flex-direction: column;
  border: 1px solid rgb(255 255 255 / 8%);
  border-radius: 22px;
  background:
    radial-gradient(circle at 48% 44%, rgb(107 91 255 / 12%), transparent 38%),
    rgb(13 16 28 / 82%);
  box-shadow: 0 24px 70px rgb(0 0 0 / 24%);
}

.preview-card__heading,
.preview-card__selection,
.preview-card__selection > div {
  display: flex;
  align-items: center;
}

.preview-card__heading {
  justify-content: space-between;
}

.preview-card__heading span {
  color: #8d82ff;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.14em;
}

h2 {
  margin: 4px 0 0;
  font-size: 20px;
}

.preview-card__heading small {
  padding: 6px 9px;
  border: 1px solid rgb(255 255 255 / 8%);
  border-radius: 999px;
  color: #8c91a8;
  background: rgb(255 255 255 / 3%);
  font-size: 11px;
}

.preview-ring {
  position: relative;
  width: 400px;
  height: 400px;
  margin: auto;
  flex: 0 0 auto;
}

.preview-ring__orbit {
  position: absolute;
  top: 50%;
  left: 50%;
  border: 1px solid rgb(255 255 255 / 8%);
  border-radius: 50%;
  transform: translate(-50%, -50%);
}

.preview-ring__orbit--outer {
  width: 336px;
  height: 336px;
  border-style: dashed;
}

.preview-ring__orbit--inner {
  width: 166px;
  height: 166px;
  border-color: rgb(139 124 255 / 15%);
}

.preview-item {
  --preview-x: 0px;
  --preview-y: 0px;
  --preview-accent: #8b7cff;
  position: absolute;
  z-index: 2;
  top: 50%;
  left: 50%;
  display: grid;
  width: 62px;
  height: 62px;
  padding: 0;
  place-items: center;
  border: 1px solid color-mix(in srgb, var(--preview-accent) 34%, rgb(255 255 255 / 8%));
  border-radius: 19px;
  color: color-mix(in srgb, var(--preview-accent) 64%, white);
  background: linear-gradient(145deg, rgb(37 41 61 / 96%), rgb(15 18 31 / 98%));
  box-shadow: 0 10px 24px rgb(0 0 0 / 28%);
  cursor: pointer;
  touch-action: none;
  transform: translate(calc(-50% + var(--preview-x)), calc(-50% + var(--preview-y)));
  transition: 150ms ease;
  user-select: none;
}

.preview-item.is-dragging {
  z-index: 5;
  cursor: grabbing;
  transition: transform 80ms ease;
}

.preview-item span {
  display: grid;
  width: 36px;
  height: 36px;
  place-items: center;
  border-radius: 12px;
  background: color-mix(in srgb, var(--preview-accent) 12%, transparent);
  font-size: 13px;
  font-weight: 700;
}

.preview-item:hover,
.preview-item.is-selected {
  border-color: color-mix(in srgb, var(--preview-accent) 78%, white);
  box-shadow:
    0 0 0 4px color-mix(in srgb, var(--preview-accent) 14%, transparent),
    0 12px 28px color-mix(in srgb, var(--preview-accent) 22%, transparent);
  transform: translate(calc(-50% + var(--preview-x)), calc(-50% + var(--preview-y))) scale(1.09);
}

.preview-center {
  position: absolute;
  top: 50%;
  left: 50%;
  display: flex;
  width: 122px;
  height: 122px;
  padding: 18px;
  align-items: center;
  justify-content: center;
  flex-direction: column;
  border: 1px solid rgb(255 255 255 / 11%);
  border-radius: 50%;
  text-align: center;
  background: linear-gradient(145deg, #24283c, #0d101d);
  box-shadow: 0 14px 34px rgb(0 0 0 / 34%);
  transform: translate(-50%, -50%);
}

.preview-center:disabled {
  cursor: default;
}

.preview-center.is-back {
  cursor: pointer;
}

.preview-center.is-back:hover,
.preview-center.is-back:focus-visible {
  border-color: rgb(139 124 255 / 55%);
  box-shadow:
    0 0 0 4px rgb(139 124 255 / 10%),
    0 14px 34px rgb(0 0 0 / 34%);
}

.preview-center strong {
  color: #b7afff;
  font-size: 10px;
  letter-spacing: 0.18em;
}

.preview-center span {
  overflow: hidden;
  width: 88px;
  margin-top: 6px;
  color: #8f94aa;
  font-size: 10px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.preview-card__selection {
  min-height: 50px;
  padding-top: 16px;
  justify-content: space-between;
  border-top: 1px solid rgb(255 255 255 / 7%);
  gap: 16px;
}

.preview-card__selection > div {
  min-width: 0;
  gap: 8px;
}

.preview-card__selection strong {
  overflow: hidden;
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.preview-card__selection small {
  color: #73798f;
  font-size: 10px;
}

.preview-card__dot {
  width: 7px;
  height: 7px;
  flex: 0 0 auto;
  border-radius: 50%;
  box-shadow: 0 0 9px currentcolor;
}

.preview-card__add {
  display: flex;
  padding: 9px 13px;
  align-items: center;
  border: 1px solid rgb(139 124 255 / 28%);
  border-radius: 10px;
  color: #bdb7ff;
  background: rgb(139 124 255 / 10%);
  cursor: pointer;
  gap: 7px;
  white-space: nowrap;
}

.preview-card__add span {
  font-size: 18px;
  line-height: 0;
}

.preview-card__add:disabled {
  cursor: not-allowed;
  opacity: 0.42;
}
</style>
