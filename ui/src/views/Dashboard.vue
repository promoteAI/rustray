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
import { ref, onMounted } from 'vue'
import SystemOverview from '@/components/SystemOverview.vue'
import CPUUsageChart from '@/components/charts/CPUUsageChart.vue'
import MemoryUsageChart from '@/components/charts/MemoryUsageChart.vue'
import NetworkThroughputChart from '@/components/charts/NetworkThroughputChart.vue'
import StorageUsageChart from '@/components/charts/StorageUsageChart.vue'
import TaskQueue from '@/components/TaskQueue.vue'
import TaskExecutionStatus from '@/components/TaskExecutionStatus.vue'

// 获取系统监控数据
const fetchSystemMetrics = async () => {
  try {
    // 调用后端API获取实��监控数据
    const response = await axios.get('/api/system/metrics')
    // 处理和更新数据
  } catch (error) {
    console.error('获取系统指标失败', error)
  }
}

onMounted(() => {
  // 定期获取系统指标
  setInterval(fetchSystemMetrics, 5000)
})
</script>

<style scoped>
.dashboard {
  @apply p-4 space-y-4
}
</style> 