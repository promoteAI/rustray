<template>
  <div class="task-list">
    <div class="task-filters">
      <div class="filter-buttons">
        <button 
          v-for="status in ['all', 'pending', 'running', 'completed', 'failed']" 
          :key="status"
          :class="['filter-btn', { active: currentFilter === status }]"
          @click="currentFilter = status"
        >
          {{ getStatusText(status) }}
          <span class="count">{{ getStatusCount(status) }}</span>
        </button>
      </div>
      <div class="search-box">
        <input 
          type="text" 
          v-model="searchQuery" 
          placeholder="搜索任务..."
          @input="filterTasks"
        >
      </div>
    </div>

    <div class="task-table">
      <table>
        <thead>
          <tr>
            <th>ID</th>
            <th>名称</th>
            <th>类型</th>
            <th>状态</th>
            <th>进度</th>
            <th>创建时间</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="task in filteredTasks" :key="task.id" :class="task.status.toLowerCase()">
            <td>{{ task.id.slice(0, 8) }}</td>
            <td>{{ task.name }}</td>
            <td>{{ getTaskType(task.type) }}</td>
            <td>
              <span :class="['status-badge', task.status.toLowerCase()]">
                {{ getStatusText(task.status) }}
              </span>
            </td>
            <td>
              <div class="progress-bar">
                <div 
                  class="progress-inner"
                  :style="{ width: `${task.progress}%` }"
                  :class="task.status.toLowerCase()"
                ></div>
                <span class="progress-text">{{ task.progress }}%</span>
              </div>
            </td>
            <td>{{ formatTime(task.createdAt) }}</td>
            <td>
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
                <button 
                  class="action-btn delete" 
                  @click="$emit('task-action', 'delete', task.id)"
                  v-if="['COMPLETED', 'FAILED'].includes(task.status)"
                >
                  删除
                </button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <div v-if="filteredTasks.length === 0" class="empty-state">
      <div class="empty-icon">📋</div>
      <p class="empty-text">{{ getEmptyText() }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'

const props = defineProps<{
  tasks: Array<{
    id: string
    name: string
    type: string
    status: string
    progress: number
    createdAt: string
  }>
}>()

const emit = defineEmits<{
  (e: 'task-action', action: string, taskId: string): void
}>()

const currentFilter = ref('all')
const searchQuery = ref('')

const filteredTasks = computed(() => {
  let filtered = props.tasks

  // 状态过滤
  if (currentFilter.value !== 'all') {
    filtered = filtered.filter(task => 
      task.status.toLowerCase() === currentFilter.value
    )
  }

  // 搜索过滤
  if (searchQuery.value) {
    const query = searchQuery.value.toLowerCase()
    filtered = filtered.filter(task =>
      task.name.toLowerCase().includes(query) ||
      task.id.toLowerCase().includes(query)
    )
  }

  return filtered
})

const getStatusCount = (status: string) => {
  if (status === 'all') return props.tasks.length
  return props.tasks.filter(task => 
    task.status.toLowerCase() === status
  ).length
}

const getStatusText = (status: string) => {
  const statusMap: Record<string, string> = {
    all: '全部',
    pending: '等待中',
    running: '运行中',
    completed: '已完成',
    failed: '失败'
  }
  return statusMap[status.toLowerCase()] || status
}

const getTaskType = (type: string) => {
  const typeMap: Record<string, string> = {
    render: '渲染任务',
    compute: '计算任务',
    analysis: '分析任务'
  }
  return typeMap[type] || type
}

const formatTime = (time: string) => {
  return new Date(time).toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit'
  })
}

const getEmptyText = () => {
  if (searchQuery.value) {
    return '没有找到匹配的任务'
  }
  if (currentFilter.value === 'all') {
    return '暂无任务'
  }
  return `暂无${getStatusText(currentFilter.value)}的任务`
}
</script>

<style scoped>
.task-list {
  background: white;
  border-radius: 8px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  overflow: hidden;
}

.task-filters {
  padding: 16px;
  border-bottom: 1px solid #e5e7eb;
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  flex-wrap: wrap;
}

.filter-buttons {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.filter-btn {
  padding: 6px 12px;
  border: 1px solid #e5e7eb;
  border-radius: 6px;
  background: white;
  color: #6b7280;
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  align-items: center;
  gap: 6px;
}

.filter-btn:hover {
  border-color: #2563eb;
  color: #2563eb;
}

.filter-btn.active {
  background: #2563eb;
  border-color: #2563eb;
  color: white;
}

.count {
  background: rgba(0, 0, 0, 0.1);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12px;
}

.search-box input {
  padding: 8px 12px;
  border: 1px solid #e5e7eb;
  border-radius: 6px;
  font-size: 14px;
  width: 200px;
  transition: border-color 0.2s;
}

.search-box input:focus {
  outline: none;
  border-color: #2563eb;
}

.task-table {
  overflow-x: auto;
}

table {
  width: 100%;
  border-collapse: collapse;
}

th, td {
  padding: 12px 16px;
  text-align: left;
  border-bottom: 1px solid #e5e7eb;
}

th {
  background: #f9fafb;
  font-weight: 500;
  color: #374151;
  white-space: nowrap;
}

td {
  color: #1f2937;
}

.status-badge {
  padding: 4px 8px;
  border-radius: 4px;
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

.progress-bar {
  width: 120px;
  height: 6px;
  background: #e5e7eb;
  border-radius: 3px;
  overflow: hidden;
  position: relative;
}

.progress-inner {
  height: 100%;
  transition: width 0.3s ease;
}

.progress-inner.pending {
  background: #d97706;
}

.progress-inner.running {
  background: #2563eb;
}

.progress-inner.completed {
  background: #059669;
}

.progress-inner.failed {
  background: #dc2626;
}

.progress-text {
  position: absolute;
  right: -30px;
  top: -4px;
  font-size: 12px;
  color: #6b7280;
}

.task-actions {
  display: flex;
  gap: 8px;
}

.action-btn {
  padding: 4px 8px;
  border: none;
  border-radius: 4px;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: background-color 0.2s;
  white-space: nowrap;
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
  .task-filters {
    flex-direction: column;
    align-items: stretch;
  }

  .search-box input {
    width: 100%;
  }

  .task-actions {
    flex-direction: column;
  }

  .action-btn {
    width: 100%;
  }
}
</style> 