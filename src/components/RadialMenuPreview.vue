<script setup lang="ts">
import { computed, ref } from 'vue'
import { itemStreamToggleState } from '../composables/useObsStreamStatus'
import type { MenuItem, ObsStreamStatus } from '../types/menu'
import MenuItemIcon from './MenuItemIcon.vue'

const props = defineProps<{
  items: MenuItem[]
  selectedId: string | null
  maxItems: number
  groupLabel: string | null
  breadcrumbLabel: string
  menuSize: number
  obsStreamStatus?: ObsStreamStatus | null
}>()

const emit = defineEmits<{
  select: [id: string]
  add: []
  reorder: [fromIndex: number, toIndex: number]
  openGroup: [id: string]
  back: []
  'update:menuSize': [size: number]
}>()

const ring = ref<HTMLElement | null>(null)
const draggingId = ref<string | null>(null)
const dragTargetIndex = ref<number | null>(null)
const dragStart = ref({ x: 0, y: 0 })
const didDrag = ref(false)

const selectedItem = computed(() =>
  props.items.find((item) => item.id === props.selectedId),
)

const sizeRatio = computed(() => Math.min(100, Math.max(0, props.menuSize)) / 100)
const previewRadius = computed(() => 118 + sizeRatio.value * 62)
const previewOuterSize = computed(() => previewRadius.value * 2 + 36)
const previewButtonSize = computed(() => 46 + sizeRatio.value * 18)
const previewIconSize = computed(() => 28 + sizeRatio.value * 10)
const ringStyle = computed(() => ({
  '--preview-outer-size': `${previewOuterSize.value}px`,
  '--preview-item-size': `${previewButtonSize.value}px`,
  '--preview-item-radius': `${14 + sizeRatio.value * 6}px`,
  '--preview-icon-size': `${previewIconSize.value}px`,
  '--preview-icon-radius': `${9 + sizeRatio.value * 5}px`,
}))

function itemPosition(index: number) {
  const positionIndex = props.items[index]?.id === draggingId.value
    ? (dragTargetIndex.value ?? index)
    : index
  const angle = (positionIndex / props.items.length) * Math.PI * 2 - Math.PI / 2
  const radius = previewRadius.value

  return {
    '--preview-x': `${Math.cos(angle) * radius}px`,
    '--preview-y': `${Math.sin(angle) * radius}px`,
    '--preview-accent': props.items[index]?.accent ?? '#8b7cff',
  }
}

function updateMenuSize(event: Event) {
  const target = event.target
  if (!(target instanceof HTMLInputElement)) return
  emit('update:menuSize', Number(target.value))
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

function itemHint(item: MenuItem) {
  return itemStreamToggleState(item, props.obsStreamStatus ?? null)?.hint ?? item.hint
}

function itemStreamState(item: MenuItem) {
  return itemStreamToggleState(item, props.obsStreamStatus ?? null)
}

function showStreamLed(item: MenuItem) {
  const state = itemStreamState(item)
  return Boolean(state?.active || state?.showInactiveLed)
}
</script>

<template>
  <section class="preview-card" aria-labelledby="preview-title">
    <div class="preview-card__heading">
      <div>
        <span>{{ breadcrumbLabel }}</span>
        <h2 id="preview-title">Preview radial</h2>
      </div>
      <small>{{ items.length }} {{ items.length === 1 ? 'item' : 'itens' }}</small>
    </div>

    <div ref="ring" class="preview-ring" :style="ringStyle">
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
          'has-stream-status': itemStreamState(item),
          'is-stream-active': itemStreamState(item)?.active,
        }"
        :style="itemPosition(index)"
        :aria-label="`Selecionar ${item.label}`"
        :title="`${item.label} — ${itemHint(item)}`"
        @click="activateItem(item)"
        @pointerdown="startDrag($event, item, index)"
        @pointermove="moveDrag"
        @pointerup="finishDrag"
        @pointercancel="finishDrag"
      >
        <i
          v-if="showStreamLed(item)"
          class="preview-item__stream-led"
          aria-hidden="true"
        />
        <span>
          <MenuItemIcon :item="item" :fixed-icon-override="itemStreamState(item)?.icon" />
        </span>
      </button>

      <button
        type="button"
        class="preview-center"
        :class="{ 'is-back': groupLabel }"
        :disabled="!groupLabel"
        :aria-label="groupLabel ? 'Voltar um nível' : undefined"
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

    <label class="preview-size-control">
      <span>
        <strong>Tamanho do menu</strong>
        <small>{{ menuSize }}%</small>
      </span>
      <input
        type="range"
        min="0"
        max="100"
        step="1"
        :value="menuSize"
        aria-label="Tamanho do menu radial"
        @input="updateMenuSize"
      >
    </label>

    <div class="preview-card__selection">
      <div>
        <span class="preview-card__dot" :style="{ background: selectedItem?.accent }" />
        <strong>{{ selectedItem?.label ?? (items.length ? 'Nenhum item selecionado' : 'Menu vazio') }}</strong>
        <small v-if="selectedItem">{{ itemHint(selectedItem) }}</small>
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
  width: var(--preview-outer-size, 272px);
  height: var(--preview-outer-size, 272px);
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
  width: var(--preview-item-size, 54px);
  height: var(--preview-item-size, 54px);
  padding: 0;
  place-items: center;
  border: 1px solid color-mix(in srgb, var(--preview-accent) 34%, rgb(255 255 255 / 8%));
  border-radius: var(--preview-item-radius, 16px);
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
  width: var(--preview-icon-size, 32px);
  height: var(--preview-icon-size, 32px);
  place-items: center;
  border-radius: var(--preview-icon-radius, 10px);
  background: color-mix(in srgb, var(--preview-accent) 12%, transparent);
  font-size: 13px;
  font-weight: 700;
}

.preview-item__stream-led {
  position: absolute;
  top: 8px;
  right: 8px;
  width: 6px;
  height: 6px;
  border: 1px solid rgb(255 255 255 / 14%);
  border-radius: 50%;
  background: rgb(255 255 255 / 13%);
  box-shadow: inset 0 1px 1px rgb(255 255 255 / 12%);
  opacity: 0.5;
  pointer-events: none;
}

.preview-item.is-stream-active .preview-item__stream-led {
  border-color: rgb(255 134 154 / 72%);
  background: #ff3858;
  box-shadow:
    0 0 0 3px rgb(255 56 88 / 9%),
    0 0 10px rgb(255 56 88 / 68%);
  opacity: 1;
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

.preview-size-control {
  display: flex;
  margin: 0 0 16px;
  align-items: center;
  gap: 16px;
}

.preview-size-control span {
  display: flex;
  width: 132px;
  flex: 0 0 auto;
  align-items: baseline;
  justify-content: space-between;
  gap: 8px;
}

.preview-size-control strong {
  color: #d8d5ff;
  font-size: 11px;
  font-weight: 600;
}

.preview-size-control small {
  color: #8c91a8;
  font-size: 10px;
  font-variant-numeric: tabular-nums;
}

.preview-size-control input {
  width: 100%;
  height: 16px;
  accent-color: #8b7cff;
  cursor: pointer;
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
