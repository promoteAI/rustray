<template>
  <div class="dashboard">
    <header class="dashboard-header">
      <div class="header-left">
        <h2>RustRay 系统仪表盘</h2>
        <div class="system-status" :class="{ active: systemRunning }">
          <span class="status-dot"></span>
          系统{{ systemRunning ? '运行中' : '已停止' }}
        </div>
      </div>
      <div class="header-right">
        <div class="refresh-info">
          <span class="refresh-text">自动刷新: {{ refreshInterval }}秒</span>
          <button class="refresh-btn" @click="manualRefresh" :disabled="isRefreshing">
            <i class="refresh-icon" :class="{ 'is-loading': isRefreshing }">⟳</i>
            刷新
          </button>
        </div>
      </div>
    </header>

    <div class="dashboard-grid">
      <!-- 系统状态卡片 -->
      <div class="status-card">
        <div class="card-header">
          <h3>系统状态</h3>
        </div>
        <div class="status-content">
          <div class="status-item">
            <div class="status-label">节点数量</div>
            <div class="status-value">{{ nodeCount }}</div>
          </div>
          <div class="status-item">
            <div class="status-label">运行任务</div>
            <div class="status-value">{{ runningTasks }}</div>
          </div>
          <div class="status-item">
            <div class="status-label">系统负载</div>
            <div class="status-value">{{ systemLoad }}</div>
          </div>
        </div>
      </div>

      <!-- CPU使用率卡片 -->
      <div class="metric-card">
        <CPUUsageChart ref="cpuChart" />
      </div>

      <!-- 内存使用率卡片 -->
      <div class="metric-card">
        <MemoryUsageChart ref="memoryChart" />
      </div>

      <!-- 网络使用率卡片 -->
      <div class="metric-card">
        <NetworkUsageChart ref="networkChart" />
      </div>
    </div>

    <!-- 任务队列面板 -->
    <div class="task-panel">
      <div class="panel-header">
        <h3>任务队列</h3>
        <div class="panel-actions">
          <select v-model="taskFilter" class="task-filter">
            <option value="all">全部任务</option>
            <option value="running">运行中</option>
            <option value="pending">等待中</option>
            <option value="completed">已完成</option>
            <option value="failed">失败</option>
          </select>
          <button class="action-btn" @click="clearCompletedTasks">
            清理已完成
          </button>
        </div>
      </div>
      <TaskQueueStatus 
        ref="taskQueue"
        :filter="taskFilter"
        @task-action="handleTaskAction"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import CPUUsageChart from '../components/charts/CPUUsageChart.vue'
import MemoryUsageChart from '../components/charts/MemoryUsageChart.vue'
import NetworkUsageChart from '../components/charts/NetworkUsageChart.vue'
import TaskQueueStatus from '../components/TaskQueueStatus.vue'
import axios from 'axios'

// 状态变量
const systemRunning = ref(true)
const nodeCount = ref(0)
const runningTasks = ref(0)
const systemLoad = ref('0%')
const refreshInterval = ref(5)
const isRefreshing = ref(false)
const taskFilter = ref('all')

// 组件引用
const cpuChart = ref(null)
const memoryChart = ref(null)
const networkChart = ref(null)
const taskQueue = ref(null)

// 刷新系统状态
const refreshSystemStatus = async () => {
  try {
    const response = await axios.get('/api/system/status')
    const { running, nodes, tasks, load } = response.data
    systemRunning.value = running
    nodeCount.value = nodes
    runningTasks.value = tasks
    systemLoad.value = load
  } catch (error) {
    console.error('获取系统状态失败:', error)
  }
}

// 手动刷新
const manualRefresh = async () => {
  if (isRefreshing.value) return
  
  isRefreshing.value = true
  try {
    await Promise.all([
      refreshSystemStatus(),
      cpuChart.value?.refresh(),
      memoryChart.value?.refresh(),
      networkChart.value?.refresh(),
      taskQueue.value?.refresh()
    ])
  } catch (error) {
    console.error('刷新失败:', error)
  } finally {
    isRefreshing.value = false
  }
}

// 处理任务操作
const handleTaskAction = async (action: string, taskId: string) => {
  try {
    await axios.post(`/api/tasks/${taskId}/${action}`)
    await taskQueue.value?.refresh()
  } catch (error) {
    console.error('任务操作失败:', error)
  }
}

// 清理已完成任务
const clearCompletedTasks = async () => {
  try {
    await axios.post('/api/tasks/clear-completed')
    await taskQueue.value?.refresh()
  } catch (error) {
    console.error('清理任务失败:', error)
  }
}

// 自动刷新定时器
let refreshTimer: number | null = null

onMounted(() => {
  refreshSystemStatus()
  refreshTimer = setInterval(manualRefresh, refreshInterval.value * 1000)
})

onUnmounted(() => {
  if (refreshTimer) {
    clearInterval(refreshTimer)
  }
})
</script>

<style scoped>
.dashboard {
  padding: 24px;
  background: #f0f2f5;
  min-height: 100vh;
}

.dashboard-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
  padding: 16px 24px;
  background: white;
  border-radius: 8px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.05);
}

.header-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

h2 {
  margin: 0;
  color: #1f2937;
  font-size: 24px;
  font-weight: 600;
}

.system-status {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  background: #f3f4f6;
  border-radius: 16px;
  font-size: 14px;
  color: #6b7280;
}

.system-status.active {
  background: #ecfdf5;
  color: #059669;
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: currentColor;
}

.refresh-info {
  display: flex;
  align-items: center;
  gap: 12px;
}

.refresh-text {
  color: #6b7280;
  font-size: 14px;
}

.refresh-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  border: none;
  border-radius: 6px;
  background: #3b82f6;
  color: white;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;
}

.refresh-btn:hover:not(:disabled) {
  background: #2563eb;
}

.refresh-btn:disabled {
  opacity: 0.7;
  cursor: not-allowed;
}

.refresh-icon {
  font-size: 16px;
  transition: transform 0.3s;
}

.refresh-icon.is-loading {
  animation: spin 1s linear infinite;
}

.dashboard-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(350px, 1fr));
  gap: 24px;
  margin-bottom: 24px;
}

.status-card, .metric-card {
  background: white;
  border-radius: 8px;
  padding: 20px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.05);
  transition: transform 0.3s, box-shadow 0.3s;
}

.status-card:hover, .metric-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.1);
}

.card-header {
  margin-bottom: 16px;
}

.card-header h3 {
  margin: 0;
  color: #1f2937;
  font-size: 18px;
  font-weight: 500;
}

.status-content {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
  gap: 16px;
}

.status-item {
  text-align: center;
}

.status-label {
  color: #6b7280;
  font-size: 14px;
  margin-bottom: 8px;
}

.status-value {
  color: #1f2937;
  font-size: 24px;
  font-weight: 600;
}

.task-panel {
  background: white;
  border-radius: 8px;
  padding: 20px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.05);
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.panel-header h3 {
  margin: 0;
  color: #1f2937;
  font-size: 18px;
  font-weight: 500;
}

.panel-actions {
  display: flex;
  gap: 12px;
}

.task-filter {
  padding: 8px 12px;
  border: 1px solid #e5e7eb;
  border-radius: 6px;
  background: white;
  color: #374151;
  font-size: 14px;
  cursor: pointer;
}

.action-btn {
  padding: 8px 16px;
  border: none;
  border-radius: 6px;
  background: #ef4444;
  color: white;
  font-size: 14px;
  cursor: pointer;
  transition: background 0.2s;
}

.action-btn:hover {
  background: #dc2626;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

@media (max-width: 768px) {
  .dashboard {
    padding: 16px;
  }

  .dashboard-header {
    flex-direction: column;
    gap: 16px;
    padding: 16px;
  }

  .header-left {
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }

  .dashboard-grid {
    grid-template-columns: 1fr;
    gap: 16px;
  }

  .panel-header {
    flex-direction: column;
    gap: 12px;
  }

  .panel-actions {
    width: 100%;
    justify-content: space-between;
  }
}
</style> 