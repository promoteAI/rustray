<template>
  <div class="system-monitor">
    <!-- 系统状态卡片 -->
    <div class="monitor-grid">
      <!-- CPU使用率 -->
      <div class="monitor-card">
        <div class="card-header">
          <h3>CPU使用率</h3>
          <span class="card-value">{{ cpuUsage }}%</span>
        </div>
        <div class="chart-container">
          <div class="chart-wrapper">
            <div class="progress-ring">
              <svg class="progress" :width="size" :height="size">
                <circle
                  class="progress-ring-circle-bg"
                  :stroke-width="strokeWidth"
                  :r="radius"
                  :cx="center"
                  :cy="center"
                />
                <circle
                  class="progress-ring-circle"
                  :stroke-width="strokeWidth"
                  :r="radius"
                  :cx="center"
                  :cy="center"
                  :style="{ strokeDashoffset: progressOffset('cpu') }"
                />
              </svg>
              <div class="progress-text">
                <span class="value">{{ cpuUsage }}%</span>
                <span class="label">使用率</span>
              </div>
            </div>
          </div>
          <div class="chart-info">
            <div class="info-item">
              <span class="label">核心数</span>
              <span class="value">{{ cpuCores }}</span>
            </div>
            <div class="info-item">
              <span class="label">平均负载</span>
              <span class="value">{{ cpuLoad }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- 内存使用率 -->
      <div class="monitor-card">
        <div class="card-header">
          <h3>内存使用率</h3>
          <span class="card-value">{{ memoryUsage }}%</span>
        </div>
        <div class="chart-container">
          <div class="chart-wrapper">
            <div class="progress-ring">
              <svg class="progress" :width="size" :height="size">
                <circle
                  class="progress-ring-circle-bg"
                  :stroke-width="strokeWidth"
                  :r="radius"
                  :cx="center"
                  :cy="center"
                />
                <circle
                  class="progress-ring-circle memory"
                  :stroke-width="strokeWidth"
                  :r="radius"
                  :cx="center"
                  :cy="center"
                  :style="{ strokeDashoffset: progressOffset('memory') }"
                />
              </svg>
              <div class="progress-text">
                <span class="value">{{ memoryUsage }}%</span>
                <span class="label">使用率</span>
              </div>
            </div>
          </div>
          <div class="chart-info">
            <div class="info-item">
              <span class="label">总内存</span>
              <span class="value">{{ formatMemory(totalMemory) }}</span>
            </div>
            <div class="info-item">
              <span class="label">已使用</span>
              <span class="value">{{ formatMemory(usedMemory) }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- 磁盘使用率 -->
      <div class="monitor-card">
        <div class="card-header">
          <h3>磁盘使用率</h3>
          <span class="card-value">{{ diskUsage }}%</span>
        </div>
        <div class="chart-container">
          <div class="chart-wrapper">
            <div class="progress-ring">
              <svg class="progress" :width="size" :height="size">
                <circle
                  class="progress-ring-circle-bg"
                  :stroke-width="strokeWidth"
                  :r="radius"
                  :cx="center"
                  :cy="center"
                />
                <circle
                  class="progress-ring-circle disk"
                  :stroke-width="strokeWidth"
                  :r="radius"
                  :cx="center"
                  :cy="center"
                  :style="{ strokeDashoffset: progressOffset('disk') }"
                />
              </svg>
              <div class="progress-text">
                <span class="value">{{ diskUsage }}%</span>
                <span class="label">使用率</span>
              </div>
            </div>
          </div>
          <div class="chart-info">
            <div class="info-item">
              <span class="label">总容量</span>
              <span class="value">{{ formatStorage(totalStorage) }}</span>
            </div>
            <div class="info-item">
              <span class="label">已使用</span>
              <span class="value">{{ formatStorage(usedStorage) }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- 网络使用率 -->
      <div class="monitor-card">
        <div class="card-header">
          <h3>网络��用率</h3>
          <span class="card-value">{{ networkSpeed }}</span>
        </div>
        <div class="chart-container">
          <div class="chart-wrapper">
            <div class="network-chart">
              <div class="network-stats">
                <div class="stat-item">
                  <span class="label">上传</span>
                  <span class="value">{{ formatNetworkSpeed(uploadSpeed) }}</span>
                </div>
                <div class="stat-item">
                  <span class="label">下载</span>
                  <span class="value">{{ formatNetworkSpeed(downloadSpeed) }}</span>
                </div>
              </div>
              <div class="network-bars">
                <div class="bar upload" :style="{ height: `${uploadPercent}%` }"></div>
                <div class="bar download" :style="{ height: `${downloadPercent}%` }"></div>
              </div>
            </div>
          </div>
          <div class="chart-info">
            <div class="info-item">
              <span class="label">总流量</span>
              <span class="value">{{ formatTraffic(totalTraffic) }}</span>
            </div>
            <div class="info-item">
              <span class="label">连接数</span>
              <span class="value">{{ connections }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import axios from 'axios'

// 图表配置
const size = 120
const strokeWidth = 8
const radius = (size - strokeWidth) / 2
const center = size / 2
const circumference = 2 * Math.PI * radius

// 系统状态数据
const cpuUsage = ref(0)
const cpuCores = ref(0)
const cpuLoad = ref('0.00')
const memoryUsage = ref(0)
const totalMemory = ref(0)
const usedMemory = ref(0)
const diskUsage = ref(0)
const totalStorage = ref(0)
const usedStorage = ref(0)
const uploadSpeed = ref(0)
const downloadSpeed = ref(0)
const totalTraffic = ref(0)
const connections = ref(0)

// 计算属性
const networkSpeed = computed(() => {
  return `↑${formatNetworkSpeed(uploadSpeed.value)} ↓${formatNetworkSpeed(downloadSpeed.value)}`
})

const uploadPercent = computed(() => {
  return Math.min((uploadSpeed.value / 1024 / 1024) * 10, 100)
})

const downloadPercent = computed(() => {
  return Math.min((downloadSpeed.value / 1024 / 1024) * 10, 100)
})

// 格式化函数
const formatMemory = (bytes: number) => {
  const gb = bytes / 1024 / 1024 / 1024
  return `${gb.toFixed(1)} GB`
}

const formatStorage = (bytes: number) => {
  const tb = bytes / 1024 / 1024 / 1024 / 1024
  return `${tb.toFixed(1)} TB`
}

const formatNetworkSpeed = (bytesPerSecond: number) => {
  if (bytesPerSecond < 1024) {
    return `${bytesPerSecond.toFixed(1)} B/s`
  } else if (bytesPerSecond < 1024 * 1024) {
    return `${(bytesPerSecond / 1024).toFixed(1)} KB/s`
  } else {
    return `${(bytesPerSecond / 1024 / 1024).toFixed(1)} MB/s`
  }
}

const formatTraffic = (bytes: number) => {
  const gb = bytes / 1024 / 1024 / 1024
  return `${gb.toFixed(1)} GB`
}

// 计算进度环偏移量
const progressOffset = (type: 'cpu' | 'memory' | 'disk') => {
  const progress = {
    cpu: cpuUsage.value,
    memory: memoryUsage.value,
    disk: diskUsage.value
  }[type]
  
  return circumference - (progress / 100) * circumference
}

// 加载系统状态
const loadSystemStatus = async () => {
  try {
    const response = await axios.get('/api/system/status')
    const data = response.data
    
    // 更新状态
    cpuUsage.value = data.cpu.usage
    cpuCores.value = data.cpu.cores
    cpuLoad.value = data.cpu.load
    memoryUsage.value = data.memory.usage
    totalMemory.value = data.memory.total
    usedMemory.value = data.memory.used
    diskUsage.value = data.disk.usage
    totalStorage.value = data.disk.total
    usedStorage.value = data.disk.used
    uploadSpeed.value = data.network.uploadSpeed
    downloadSpeed.value = data.network.downloadSpeed
    totalTraffic.value = data.network.totalTraffic
    connections.value = data.network.connections
  } catch (error) {
    console.error('加载系统状态失败:', error)
  }
}

// 自动刷新
let refreshTimer: number | null = null

onMounted(() => {
  loadSystemStatus()
  refreshTimer = setInterval(loadSystemStatus, 2000) // 每2秒刷新一次
})

onUnmounted(() => {
  if (refreshTimer) {
    clearInterval(refreshTimer)
  }
})
</script>

<style scoped>
.system-monitor {
  padding: 24px;
}

.monitor-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 24px;
}

.monitor-card {
  background: white;
  border-radius: 8px;
  padding: 20px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

h3 {
  margin: 0;
  font-size: 16px;
  font-weight: 500;
  color: #1f2937;
}

.card-value {
  font-size: 14px;
  font-weight: 500;
  color: #6b7280;
}

.chart-container {
  display: flex;
  gap: 24px;
}

.chart-wrapper {
  flex: 1;
  display: flex;
  justify-content: center;
}

.progress-ring {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
}

.progress {
  transform: rotate(-90deg);
}

.progress-ring-circle-bg {
  fill: none;
  stroke: #e5e7eb;
}

.progress-ring-circle {
  fill: none;
  stroke: #2563eb;
  stroke-linecap: round;
  transition: stroke-dashoffset 0.3s;
}

.progress-ring-circle.memory {
  stroke: #059669;
}

.progress-ring-circle.disk {
  stroke: #d97706;
}

circle {
  stroke-dasharray: v-bind(circumference) v-bind(circumference);
  transition: stroke-dashoffset 0.3s;
}

.progress-text {
  position: absolute;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.progress-text .value {
  font-size: 20px;
  font-weight: 600;
  color: #1f2937;
}

.progress-text .label {
  font-size: 12px;
  color: #6b7280;
}

.chart-info {
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 12px;
}

.info-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.info-item .label {
  font-size: 12px;
  color: #6b7280;
}

.info-item .value {
  font-size: 14px;
  font-weight: 500;
  color: #1f2937;
}

.network-chart {
  width: 100%;
  display: flex;
  gap: 24px;
}

.network-stats {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.network-bars {
  flex: 1;
  height: 80px;
  display: flex;
  gap: 8px;
  align-items: flex-end;
}

.bar {
  flex: 1;
  background: #2563eb;
  border-radius: 4px 4px 0 0;
  transition: height 0.3s ease;
}

.bar.upload {
  background: #059669;
}

.bar.download {
  background: #2563eb;
}

@media (max-width: 640px) {
  .system-monitor {
    padding: 16px;
  }

  .monitor-grid {
    grid-template-columns: 1fr;
    gap: 16px;
  }

  .chart-container {
    flex-direction: column;
    align-items: center;
    gap: 16px;
  }

  .chart-info {
    flex-direction: row;
    justify-content: space-around;
    width: 100%;
  }
}
</style> 