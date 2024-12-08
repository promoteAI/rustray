<template>
  <div class="tasks-page">
    <header class="page-header">
      <h1>任务管理</h1>
      <div class="header-actions">
        <button class="create-task-btn" @click="showCreateTaskModal = true">
          <span class="btn-icon">+</span>
          创建任务
        </button>
      </div>
    </header>

    <TaskList 
      :tasks="tasks"
      @task-action="handleTaskAction"
    />

    <!-- 创建任务模态框 -->
    <div v-if="showCreateTaskModal" class="modal-overlay" @click="showCreateTaskModal = false">
      <div class="modal-content" @click.stop>
        <h2>创建新任务</h2>
        <form @submit.prevent="createTask" class="task-form">
          <div class="form-group">
            <label for="taskName">任务名称</label>
            <input 
              type="text" 
              id="taskName" 
              v-model="newTask.name"
              required
              placeholder="请输入任务名称"
            >
          </div>
          
          <div class="form-group">
            <label for="taskType">任务类型</label>
            <select id="taskType" v-model="newTask.type" required>
              <option value="">请选择任务类型</option>
              <option value="render">渲染任务</option>
              <option value="compute">计算任务</option>
              <option value="analysis">分析任务</option>
            </select>
          </div>
          
          <div class="form-group">
            <label for="taskPriority">优先级</label>
            <select id="taskPriority" v-model="newTask.priority" required>
              <option value="low">低</option>
              <option value="medium">中</option>
              <option value="high">高</option>
            </select>
          </div>
          
          <div class="form-group">
            <label for="taskDescription">任务描述</label>
            <textarea 
              id="taskDescription"
              v-model="newTask.description"
              rows="4"
              placeholder="请输入任务描述"
            ></textarea>
          </div>

          <div class="form-group">
            <label for="taskConfig">任务配置</label>
            <div class="config-grid">
              <div class="config-item">
                <label for="maxWorkers">最大工作线程</label>
                <input 
                  type="number" 
                  id="maxWorkers" 
                  v-model="newTask.config.maxWorkers"
                  min="1"
                  max="32"
                >
              </div>
              
              <div class="config-item">
                <label for="timeout">超时时间(分钟)</label>
                <input 
                  type="number" 
                  id="timeout" 
                  v-model="newTask.config.timeout"
                  min="1"
                  max="1440"
                >
              </div>
              
              <div class="config-item">
                <label for="retryCount">重试次数</label>
                <input 
                  type="number" 
                  id="retryCount" 
                  v-model="newTask.config.retryCount"
                  min="0"
                  max="5"
                >
              </div>
              
              <div class="config-item">
                <div class="checkbox-group">
                  <input 
                    type="checkbox" 
                    id="useGPU" 
                    v-model="newTask.config.useGPU"
                  >
                  <label for="useGPU">使用GPU加速</label>
                </div>
              </div>
            </div>
          </div>
          
          <div class="form-actions">
            <button type="button" class="cancel-btn" @click="showCreateTaskModal = false">取消</button>
            <button type="submit" class="submit-btn">创建</button>
          </div>
        </form>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import TaskList from '../components/TaskList.vue'
import axios from 'axios'

const tasks = ref([])
const showCreateTaskModal = ref(false)

const newTask = ref({
  name: '',
  type: '',
  priority: 'medium',
  description: '',
  config: {
    maxWorkers: 4,
    timeout: 30,
    retryCount: 3,
    useGPU: false
  }
})

// 加载任务列表
const loadTasks = async () => {
  try {
    const response = await axios.get('/api/tasks')
    tasks.value = response.data
  } catch (error) {
    console.error('加载任务列表失败:', error)
  }
}

// 处理任务操作
const handleTaskAction = async (action: string, taskId: string) => {
  try {
    await axios.post(`/api/tasks/${taskId}/${action}`)
    await loadTasks()
  } catch (error) {
    console.error('任务操作失败:', error)
  }
}

// 创建新任务
const createTask = async () => {
  try {
    await axios.post('/api/tasks', newTask.value)
    showCreateTaskModal.value = false
    await loadTasks()
    // 重置表单
    newTask.value = {
      name: '',
      type: '',
      priority: 'medium',
      description: '',
      config: {
        maxWorkers: 4,
        timeout: 30,
        retryCount: 3,
        useGPU: false
      }
    }
  } catch (error) {
    console.error('创建任务失败:', error)
  }
}

// 自动刷新任务列表
let refreshTimer: number | null = null

onMounted(() => {
  loadTasks()
  refreshTimer = setInterval(loadTasks, 5000) // 每5秒刷新一次
})

onUnmounted(() => {
  if (refreshTimer) {
    clearInterval(refreshTimer)
  }
})
</script>

<style scoped>
.tasks-page {
  padding: 24px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

h1 {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
  color: #1f2937;
}

.create-task-btn {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  background: #2563eb;
  color: white;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: background-color 0.2s;
}

.create-task-btn:hover {
  background: #1d4ed8;
}

.btn-icon {
  font-size: 18px;
  font-weight: bold;
}

.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  background: white;
  border-radius: 8px;
  padding: 24px;
  width: 100%;
  max-width: 600px;
  max-height: 90vh;
  overflow-y: auto;
}

.modal-content h2 {
  margin: 0 0 24px 0;
  font-size: 20px;
  color: #1f2937;
}

.task-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.form-group label {
  font-size: 14px;
  color: #374151;
  font-weight: 500;
}

.form-group input,
.form-group select,
.form-group textarea {
  padding: 8px 12px;
  border: 1px solid #d1d5db;
  border-radius: 6px;
  font-size: 14px;
  color: #1f2937;
  transition: border-color 0.2s;
}

.form-group input:focus,
.form-group select:focus,
.form-group textarea:focus {
  outline: none;
  border-color: #2563eb;
}

.config-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 16px;
}

.config-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.checkbox-group {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 8px;
}

.checkbox-group input[type="checkbox"] {
  width: 16px;
  height: 16px;
}

.form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 24px;
}

.cancel-btn,
.submit-btn {
  padding: 8px 16px;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: background-color 0.2s;
}

.cancel-btn {
  background: #f3f4f6;
  color: #374151;
}

.cancel-btn:hover {
  background: #e5e7eb;
}

.submit-btn {
  background: #2563eb;
  color: white;
}

.submit-btn:hover {
  background: #1d4ed8;
}

@media (max-width: 640px) {
  .tasks-page {
    padding: 16px;
  }

  .page-header {
    flex-direction: column;
    gap: 16px;
    align-items: stretch;
  }

  .create-task-btn {
    width: 100%;
    justify-content: center;
  }

  .modal-content {
    margin: 16px;
    padding: 16px;
  }

  .config-grid {
    grid-template-columns: 1fr;
  }
}
</style> 