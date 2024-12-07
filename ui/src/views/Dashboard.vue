<template>
  <div class="dashboard">
    <h2>RustRay 系统仪表盘</h2>
    
    <!-- 系统概览 -->
    <SystemOverview />
    
    <!-- 监控指标图表 -->
    <div class="metrics-grid">
      <div class="metric-card">
        <CPUUsageChart />
      </div>
      <div class="metric-card">
        <MemoryUsageChart />
      </div>
      <div class="metric-card">
        <NetworkUsageChart />
      </div>
      <div class="metric-card">
        <StorageUsageChart />
      </div>
    </div>

    <!-- 任务队列状态 -->
    <div class="queue-status">
      <h3>任务队列状态</h3>
      <TaskQueueStatus />
    </div>
  </div>
</template>

<script setup lang="ts">
import SystemOverview from '../components/SystemOverview.vue'
import CPUUsageChart from '../components/charts/CPUUsageChart.vue'
import MemoryUsageChart from '../components/charts/MemoryUsageChart.vue'
import NetworkUsageChart from '../components/charts/NetworkUsageChart.vue'
import StorageUsageChart from '../components/charts/StorageUsageChart.vue'
import TaskQueueStatus from '../components/TaskQueueStatus.vue'
</script>

<style scoped>
.dashboard {
  padding: 20px;
}

.metrics-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 20px;
  margin-top: 20px;
}

.metric-card {
  background: white;
  border-radius: 8px;
  padding: 15px;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.queue-status {
  margin-top: 20px;
  background: white;
  border-radius: 8px;
  padding: 15px;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

h2 {
  margin-bottom: 20px;
  color: #2c3e50;
}

h3 {
  margin-bottom: 15px;
  color: #2c3e50;
}
</style> 