<script setup lang="ts">
import type { SystemStats } from '../types/systemStats'

const props = defineProps<{
  stats: SystemStats | null
  shortcut: string
}>()

function percent(value: number | null | undefined) {
  return Number.isFinite(value) ? `${Math.round(value!)}%` : '--'
}

function formatBytes(bytes: number | null | undefined) {
  if (!Number.isFinite(bytes)) return '--'
  if (bytes === 0) return '0B'

  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const unitIndex = Math.min(
    Math.floor(Math.log(bytes!) / Math.log(1024)),
    units.length - 1,
  )
  const value = bytes! / 1024 ** unitIndex
  const precision = value < 10 && unitIndex > 0 ? 1 : 0

  return `${value.toFixed(precision)}${units[unitIndex]}`
}

function rate(bytes: number | null | undefined) {
  if (!Number.isFinite(bytes)) return '--'
  if (bytes! >= 1024 ** 2) return `${(bytes! / 1024 ** 2).toFixed(1)} MB/s`
  return `${Math.round(bytes! / 1024)} KB/s`
}
</script>

<template>
  <div class="system-status" aria-live="polite">
    <div class="system-status__brand">
      <span class="system-status__pulse" />
      <strong>ORBIT</strong>
    </div>

    <div class="system-status__pair">
      <span>CPU <b>{{ percent(props.stats?.cpuPercent) }}</b></span>
      <span>RAM <b>{{ percent(props.stats?.memoryPercent) }}</b></span>
    </div>

    <div class="system-status__disk">
      <span>{{ props.stats?.diskLabel ?? 'DISCO' }}</span>
      <b>{{ formatBytes(props.stats?.diskUsedBytes) }}</b>
      <small>/ {{ formatBytes(props.stats?.diskTotalBytes) }}</small>
    </div>

    <div class="system-status__network">
      <span class="is-download">↓ {{ rate(props.stats?.downloadBytesPerSecond) }}</span>
      <span class="is-upload">↑ {{ rate(props.stats?.uploadBytesPerSecond) }}</span>
    </div>

    <small class="system-status__shortcut">{{ shortcut }}</small>
  </div>
</template>
