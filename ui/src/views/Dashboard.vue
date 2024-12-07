<template>
  <div class="dashboard">
    <!-- 系统总览 -->
    <div class="overview">
      <SystemOverview />
    </div>

    <!-- 资源监控 -->
    <div class="resource-monitor grid grid-cols-2 gap-4">
      <CPUUsageChart />
      <MemoryUsageChart />
      <NetworkThroughputChart />
      <StorageUsageChart />
    </div>

    <!-- 任务管理 -->
    <div class="task-management">
      <TaskQueue />
      <TaskExecutionStatus />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import SystemOverview from '@/components/SystemOverview.vue'
import CPUUsageChart from '@/components/charts/CPUUsageChart.vue'
import MemoryUsageChart from '@/components/charts/MemoryUsageChart.vue'
import NetworkThroughputChart from '@/components/charts/NetworkThroughputChart.vue'
import StorageUsageChart from '@/components/charts/StorageUsageChart.vue'
import TaskQueue from '@/components/TaskQueue.vue'
import TaskExecutionStatus from '@/components/TaskExecutionStatus.vue'
import axios from 'axios'

const systemMetrics = ref(null)
const isLoading = ref(false)
const error = ref(null)

const fetchSystemMetrics = async () => {
  isLoading.value = true
  error.value = null
  try {
    const response = await axios.get('/api/system/metrics')
    systemMetrics.value = response.data
  } catch (err) {
    error.value = err
    console.error('获取系统指标失败', err)
  } finally {
    isLoading.value = false
  }
}

let intervalId: number

onMounted(() => {
  fetchSystemMetrics()
  intervalId = setInterval(fetchSystemMetrics, 5000)
})

onUnmounted(() => {
  clearInterval(intervalId)
})
</script>

<style scoped>
.dashboard {
  @apply p-4 space-y-4
}
</style> 