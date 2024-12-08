<template>
  <div class="settings-page">
    <header class="page-header">
      <h1>系统设置</h1>
    </header>

    <!-- 系统监控 -->
    <SystemMonitor />

    <div class="settings-grid">
      <!-- 系统配置 -->
      <section class="settings-section">
        <h2>系统配置</h2>
        <div class="settings-form">
          <div class="form-group">
            <label for="maxWorkers">最大工作线程数</label>
            <input 
              type="number" 
              id="maxWorkers" 
              v-model="settings.maxWorkers"
              min="1"
              max="32"
            >
          </div>
          
          <div class="form-group">
            <label for="taskTimeout">任务超时时间(分钟)</label>
            <input 
              type="number" 
              id="taskTimeout" 
              v-model="settings.taskTimeout"
              min="1"
              max="1440"
            >
          </div>
          
          <div class="form-group">
            <label for="logLevel">日志级别</label>
            <select id="logLevel" v-model="settings.logLevel">
              <option value="debug">Debug</option>
              <option value="info">Info</option>
              <option value="warn">Warn</option>
              <option value="error">Error</option>
            </select>
          </div>
        </div>
      </section>

      <!-- 性能设置 -->
      <section class="settings-section">
        <h2>性能设置</h2>
        <div class="settings-form">
          <div class="form-group">
            <label for="maxMemory">最大内存使用(GB)</label>
            <input 
              type="number" 
              id="maxMemory" 
              v-model="settings.maxMemory"
              min="1"
              max="128"
            >
          </div>
          
          <div class="form-group">
            <label for="cpuPriority">CPU优先级</label>
            <select id="cpuPriority" v-model="settings.cpuPriority">
              <option value="low">低</option>
              <option value="normal">正常</option>
              <option value="high">高</option>
            </select>
          </div>
          
          <div class="form-group">
            <div class="checkbox-group">
              <input 
                type="checkbox" 
                id="enableGPU" 
                v-model="settings.enableGPU"
              >
              <label for="enableGPU">启用GPU加速</label>
            </div>
          </div>
        </div>
      </section>

      <!-- 网络设置 -->
      <section class="settings-section">
        <h2>网络设置</h2>
        <div class="settings-form">
          <div class="form-group">
            <label for="apiPort">API端口</label>
            <input 
              type="number" 
              id="apiPort" 
              v-model="settings.apiPort"
              min="1024"
              max="65535"
            >
          </div>
          
          <div class="form-group">
            <label for="maxConnections">最大连接数</label>
            <input 
              type="number" 
              id="maxConnections" 
              v-model="settings.maxConnections"
              min="1"
              max="1000"
            >
          </div>
          
          <div class="form-group">
            <div class="checkbox-group">
              <input 
                type="checkbox" 
                id="enableSSL" 
                v-model="settings.enableSSL"
              >
              <label for="enableSSL">启用SSL</label>
            </div>
          </div>
        </div>
      </section>

      <!-- 存储设置 -->
      <section class="settings-section">
        <h2>存储设置</h2>
        <div class="settings-form">
          <div class="form-group">
            <label for="dataDir">数据存储目录</label>
            <input 
              type="text" 
              id="dataDir" 
              v-model="settings.dataDir"
              placeholder="/path/to/data"
            >
          </div>
          
          <div class="form-group">
            <label for="backupInterval">备份间隔(小时)</label>
            <input 
              type="number" 
              id="backupInterval" 
              v-model="settings.backupInterval"
              min="1"
              max="168"
            >
          </div>
          
          <div class="form-group">
            <div class="checkbox-group">
              <input 
                type="checkbox" 
                id="enableCompression" 
                v-model="settings.enableCompression"
              >
              <label for="enableCompression">启用数据压缩</label>
            </div>
          </div>
        </div>
      </section>
    </div>

    <div class="settings-actions">
      <button class="reset-btn" @click="resetSettings">重置设置</button>
      <button class="save-btn" @click="saveSettings">保存设置</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import axios from 'axios'
import SystemMonitor from '../components/SystemMonitor.vue'

const settings = ref({
  // 系统配置
  maxWorkers: 4,
  taskTimeout: 30,
  logLevel: 'info',
  
  // 性能设置
  maxMemory: 8,
  cpuPriority: 'normal',
  enableGPU: true,
  
  // 网络设置
  apiPort: 3000,
  maxConnections: 100,
  enableSSL: false,
  
  // 存储设置
  dataDir: '/data',
  backupInterval: 24,
  enableCompression: true
})

// 加载设置
const loadSettings = async () => {
  try {
    const response = await axios.get('/api/settings')
    settings.value = { ...settings.value, ...response.data }
  } catch (error) {
    console.error('加载设置失败:', error)
  }
}

// 保存设置
const saveSettings = async () => {
  try {
    await axios.post('/api/settings', settings.value)
    alert('设置已保存')
  } catch (error) {
    console.error('保存设置失败:', error)
    alert('保存设置失败')
  }
}

// 重置设置
const resetSettings = async () => {
  if (confirm('确定要重置所有设置吗？')) {
    try {
      await axios.post('/api/settings/reset')
      await loadSettings()
      alert('设置已重置')
    } catch (error) {
      console.error('重置设置失败:', error)
      alert('重置设置失败')
    }
  }
}

onMounted(() => {
  loadSettings()
})
</script>

<style scoped>
.settings-page {
  padding: 24px;
}

.page-header {
  margin-bottom: 24px;
}

h1 {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
  color: #1f2937;
}

.settings-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 24px;
  margin-bottom: 24px;
}

.settings-section {
  background: white;
  border-radius: 8px;
  padding: 20px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.settings-section h2 {
  margin: 0 0 16px 0;
  font-size: 18px;
  color: #1f2937;
}

.settings-form {
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

.form-group input[type="text"],
.form-group input[type="number"],
.form-group select {
  padding: 8px 12px;
  border: 1px solid #d1d5db;
  border-radius: 6px;
  font-size: 14px;
  color: #1f2937;
  transition: border-color 0.2s;
}

.form-group input:focus,
.form-group select:focus {
  outline: none;
  border-color: #2563eb;
}

.checkbox-group {
  display: flex;
  align-items: center;
  gap: 8px;
}

.checkbox-group input[type="checkbox"] {
  width: 16px;
  height: 16px;
  border: 1px solid #d1d5db;
  border-radius: 4px;
  cursor: pointer;
}

.checkbox-group label {
  cursor: pointer;
}

.settings-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 24px;
}

.reset-btn,
.save-btn {
  padding: 8px 16px;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: background-color 0.2s;
}

.reset-btn {
  background: #f3f4f6;
  color: #374151;
}

.reset-btn:hover {
  background: #e5e7eb;
}

.save-btn {
  background: #2563eb;
  color: white;
}

.save-btn:hover {
  background: #1d4ed8;
}

@media (max-width: 640px) {
  .settings-page {
    padding: 16px;
  }

  .settings-grid {
    grid-template-columns: 1fr;
    gap: 16px;
  }

  .settings-actions {
    flex-direction: column-reverse;
    gap: 8px;
  }

  .reset-btn,
  .save-btn {
    width: 100%;
  }
}
</style> 