<template>
  <div class="task-queue-status">
    <div class="queue-stats">
      <div class="stat-item">
        <span class="label">等待中任务</span>
        <span class="value">{{ pendingTasks }}</span>
      </div>
      <div class="stat-item">
        <span class="label">运行中任务</span>
        <span class="value">{{ runningTasks }}</span>
      </div>
      <div class="stat-item">
        <span class="label">已完成任务</span>
        <span class="value">{{ completedTasks }}</span>
      </div>
      <div class="stat-item">
        <span class="label">失败任务</span>
        <span class="value">{{ failedTasks }}</span>
      </div>
    </div>

    <div class="task-list" v-if="recentTasks.length > 0">
      <h4>最近任务</h4>
      <div class="task-item" v-for="task in recentTasks" :key="task.id">
        <div class="task-info">
          <span class="task-name">{{ task.name }}</span>
          <span :class="['task-status', task.status.toLowerCase()]">{{ task.status }}</span>
        </div>
        <div class="task-details">
          <span class="task-time">{{ task.startTime }}</span>
          <span class="task-duration" v-if="task.duration">{{ task.duration }}</span>
        </div>
      </div>
    </div>
    <div class="no-tasks" v-else>
      暂无任务
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import axios from 'axios'

interface Task {
  id: string
  name: string
  status: string
  startTime: string
  duration?: string
}

const pendingTasks = ref(0)
const runningTasks = ref(0)
const completedTasks = ref(0)
const failedTasks = ref(0)
const recentTasks = ref<Task[]>([])

const fetchQueueStatus = async () => {
  try {
    const response = await axios.get('/api/tasks/status')
    const data = response.data
    
    pendingTasks.value = data.pending
    runningTasks.value = data.running
    completedTasks.value = data.completed
    failedTasks.value = data.failed
    recentTasks.value = data.recent_tasks
  } catch (error) {
    console.error('获取任务队列状态失败', error)
  }
}

onMounted(() => {
  fetchQueueStatus()
  setInterval(fetchQueueStatus, 5000)
})
</script>

<style scoped>
.task-queue-status {
  padding: 15px;
}

.queue-stats {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 15px;
  margin-bottom: 20px;
}

.stat-item {
  background: #f5f7fa;
  padding: 15px;
  border-radius: 8px;
  text-align: center;
}

.label {
  display: block;
  color: #606266;
  font-size: 14px;
  margin-bottom: 5px;
}

.value {
  font-size: 24px;
  font-weight: bold;
  color: #303133;
}

.task-list {
  margin-top: 20px;
}

.task-item {
  padding: 12px;
  border-bottom: 1px solid #ebeef5;
}

.task-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 5px;
}

.task-name {
  font-weight: 500;
  color: #303133;
}

.task-status {
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 12px;
}

.task-status.pending {
  background: #e6a23c;
  color: #fff;
}

.task-status.running {
  background: #409eff;
  color: #fff;
}

.task-status.completed {
  background: #67c23a;
  color: #fff;
}

.task-status.failed {
  background: #f56c6c;
  color: #fff;
}

.task-details {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
  color: #909399;
}

.no-tasks {
  text-align: center;
  color: #909399;
  padding: 20px;
}

h4 {
  margin: 0 0 15px 0;
  color: #303133;
}
</style> 