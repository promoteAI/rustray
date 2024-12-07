<template>
  <div class="task-queue-status">
    <!-- 任务统计 -->
    <div class="queue-stats">
      <div class="stat-item" :class="{ active: filter === 'pending' }">
        <div class="stat-icon pending">⌛</div>
        <div class="stat-info">
          <span class="stat-label">等待中</span>
          <span class="stat-value">{{ pendingTasks }}</span>
        </div>
      </div>
      <div class="stat-item" :class="{ active: filter === 'running' }">
        <div class="stat-icon running">⚡</div>
        <div class="stat-info">
          <span class="stat-label">运行中</span>
          <span class="stat-value">{{ runningTasks }}</span>
        </div>
      </div>
      <div class="stat-item" :class="{ active: filter === 'completed' }">
        <div class="stat-icon completed">✓</div>
        <div class="stat-info">
          <span class="stat-label">已完成</span>
          <span class="stat-value">{{ completedTasks }}</span>
        </div>
      </div>
      <div class="stat-item" :class="{ active: filter === 'failed' }">
        <div class="stat-icon failed">✗</div>
        <div class="stat-info">
          <span class="stat-label">失败</span>
          <span class="stat-value">{{ failedTasks }}</span>
        </div>
      </div>
    </div>

    <!-- 任务列表 -->
    <div class="task-list" v-if="filteredTasks.length > 0">
      <div v-for="task in filteredTasks" 
           :key="task.id" 
           class="task-item"
           :class="task.status.toLowerCase()">
        <div class="task-header">
          <div class="task-title">
            <span class="task-id">#{{ task.id.slice(0, 8) }}</span>
            <span class="task-name">{{ task.name }}</span>
          </div>
          <div class="task-status">
            <span class="status-badge" :class="task.status.toLowerCase()">
              {{ getStatusText(task.status) }}
            </span>
          </div>
        </div>
        
        <div class="task-body">
          <div class="task-info">
            <div class="info-item">
              <i class="info-icon">🕒</i>
              <span>开始时间: {{ formatTime(task.startTime) }}</span>
            </div>
            <div class="info-item" v-if="task.duration">
              <i class="info-icon">⌛</i>
              <span>运行时长: {{ task.duration }}</span>
            </div>
            <div class="info-item" v-if="task.progress">
              <i class="info-icon">📊</i>
              <span>进度: {{ task.progress }}%</span>
            </div>
          </div>
          
          <div class="task-actions">
            <template v-if="task.status === 'PENDING'">
              <button class="action-btn start" @click="$emit('task-action', 'start', task.id)">
                开始
              </button>
            </template>
            <template v-if="task.status === 'RUNNING'">
              <button class="action-btn pause" @click="$emit('task-action', 'pause', task.id)">
                暂停
              </button>
              <button class="action-btn stop" @click="$emit('task-action', 'stop', task.id)">
                停止
              </button>
            </template>
            <template v-if="task.status === 'FAILED'">
              <button class="action-btn retry" @click="$emit('task-action', 'retry', task.id)">
                重试
              </button>
            </template>
            <button class="action-btn delete" @click="$emit('task-action', 'delete', task.id)">
              删除
            </button>
          </div>
        </div>

        <!-- 进度条 -->
        <div v-if="task.status === 'RUNNING' && task.progress" class="progress-bar">
          <div class="progress-inner" :style="{ width: `${task.progress}%` }"></div>
        </div>
      </div>
    </div>

    <!-- 空状态 -->
    <div v-else class="empty-state">
      <div class="empty-icon">📋</div>
      <p class="empty-text">暂无{{ getFilterText() }}任务</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import axios from 'axios'

interface Task {
  id: string
  name: string
  status: string
  startTime: string
  duration?: string
  progress?: number
}

const props = defineProps<{
  filter: string
}>()

defineEmits<{
  (e: 'task-action', action: string, taskId: string): void
}>()

const pendingTasks = ref(0)
const runningTasks = ref(0)
const completedTasks = ref(0)
const failedTasks = ref(0)
const recentTasks = ref<Task[]>([])

const filteredTasks = computed(() => {
  if (props.filter === 'all') return recentTasks.value
  return recentTasks.value.filter(task => task.status.toLowerCase() === props.filter)
})

const getStatusText = (status: string): string => {
  const statusMap: Record<string, string> = {
    'PENDING': '等待中',
    'RUNNING': '运行中',
    'COMPLETED': '已完成',
    'FAILED': '已失败',
    'PAUSED': '已暂停'
  }
  return statusMap[status] || status
}

const getFilterText = (): string => {
  const filterMap: Record<string, string> = {
    'all': '',
    'pending': '等待中',
    'running': '运行中',
    'completed': '已完成',
    'failed': '失败'
  }
  return filterMap[props.filter]
}

const formatTime = (time: string): string => {
  const date = new Date(time)
  return date.toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit'
  })
}

const refresh = async () => {
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

defineExpose({
  refresh
})
</script>

<style scoped>
.task-queue-status {
  padding: 16px;
}

.queue-stats {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 16px;
  margin-bottom: 24px;
}

.stat-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px;
  background: #f9fafb;
  border-radius: 8px;
  border: 2px solid transparent;
  transition: all 0.3s;
}

.stat-item:hover {
  transform: translateY(-2px);
}

.stat-item.active {
  border-color: currentColor;
  background: #fff;
}

.stat-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border-radius: 8px;
  font-size: 20px;
}

.stat-icon.pending {
  background: #fef3c7;
  color: #d97706;
}

.stat-icon.running {
  background: #dbeafe;
  color: #2563eb;
}

.stat-icon.completed {
  background: #d1fae5;
  color: #059669;
}

.stat-icon.failed {
  background: #fee2e2;
  color: #dc2626;
}

.stat-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.stat-label {
  color: #6b7280;
  font-size: 14px;
}

.stat-value {
  color: #111827;
  font-size: 24px;
  font-weight: 600;
}

.task-list {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.task-item {
  background: #fff;
  border-radius: 8px;
  border: 1px solid #e5e7eb;
  overflow: hidden;
  transition: all 0.3s;
}

.task-item:hover {
  border-color: #d1d5db;
  box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1);
}

.task-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px;
  border-bottom: 1px solid #e5e7eb;
}

.task-title {
  display: flex;
  align-items: center;
  gap: 8px;
}

.task-id {
  color: #6b7280;
  font-family: monospace;
  font-size: 14px;
}

.task-name {
  color: #111827;
  font-weight: 500;
}

.status-badge {
  padding: 4px 12px;
  border-radius: 9999px;
  font-size: 12px;
  font-weight: 500;
}

.status-badge.pending {
  background: #fef3c7;
  color: #d97706;
}

.status-badge.running {
  background: #dbeafe;
  color: #2563eb;
}

.status-badge.completed {
  background: #d1fae5;
  color: #059669;
}

.status-badge.failed {
  background: #fee2e2;
  color: #dc2626;
}

.task-body {
  padding: 16px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex-wrap: wrap;
  gap: 16px;
}

.task-info {
  display: flex;
  flex-wrap: wrap;
  gap: 16px;
}

.info-item {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #6b7280;
  font-size: 14px;
}

.info-icon {
  font-size: 16px;
}

.task-actions {
  display: flex;
  gap: 8px;
}

.action-btn {
  padding: 6px 12px;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.action-btn.start {
  background: #059669;
  color: white;
}

.action-btn.start:hover {
  background: #047857;
}

.action-btn.pause {
  background: #d97706;
  color: white;
}

.action-btn.pause:hover {
  background: #b45309;
}

.action-btn.stop {
  background: #dc2626;
  color: white;
}

.action-btn.stop:hover {
  background: #b91c1c;
}

.action-btn.retry {
  background: #2563eb;
  color: white;
}

.action-btn.retry:hover {
  background: #1d4ed8;
}

.action-btn.delete {
  background: #ef4444;
  color: white;
}

.action-btn.delete:hover {
  background: #dc2626;
}

.progress-bar {
  height: 4px;
  background: #e5e7eb;
  overflow: hidden;
}

.progress-inner {
  height: 100%;
  background: #2563eb;
  transition: width 0.3s ease;
}

.empty-state {
  padding: 48px;
  text-align: center;
}

.empty-icon {
  font-size: 48px;
  margin-bottom: 16px;
  color: #9ca3af;
}

.empty-text {
  color: #6b7280;
  font-size: 16px;
  margin: 0;
}

@media (max-width: 768px) {
  .queue-stats {
    grid-template-columns: repeat(2, 1fr);
  }

  .task-body {
    flex-direction: column;
    align-items: stretch;
  }

  .task-actions {
    justify-content: flex-end;
  }
}
</style> 