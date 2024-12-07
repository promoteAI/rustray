// API基础路径
export const API_BASE_URL = import.meta.env.VITE_API_BASE_URL || 'http://localhost:3000'

// API路径配置
export const API_ROUTES = {
  // 系统相关
  SYSTEM_OVERVIEW: '/api/system/overview',
  SYSTEM_STATUS: '/api/system/status',
  
  // 监控指标
  METRICS: {
    CPU: '/api/metrics/cpu',
    MEMORY: '/api/metrics/memory',
    NETWORK: '/api/metrics/network',
    STORAGE: '/api/metrics/storage'
  },
  
  // 任务相关
  TASKS: {
    STATUS: '/api/tasks/status',
    CLEAR_COMPLETED: '/api/tasks/clear-completed',
    ACTION: (taskId: string, action: string) => `/api/tasks/${taskId}/${action}`
  }
}

// 创建完整的API URL
export const createApiUrl = (path: string): string => {
  return `${API_BASE_URL}${path}`
}

// axios配置
export const axiosConfig = {
  baseURL: API_BASE_URL,
  timeout: 10000,
  headers: {
    'Content-Type': 'application/json'
  }
} 