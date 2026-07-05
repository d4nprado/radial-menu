export type SystemStats = {
  cpuPercent: number | null
  memoryPercent: number | null
  diskUsedBytes: number | null
  diskTotalBytes: number | null
  diskLabel: string | null
  downloadBytesPerSecond: number | null
  uploadBytesPerSecond: number | null
}
