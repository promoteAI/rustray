import axios from 'axios'

const request = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL || '/api',
  timeout: 10000
})

request.interceptors.request.use(
  config => {
    // 添加 token
    return config
  },
  error => Promise.reject(error)
)

request.interceptors.response.use(
  response => response.data,
  error => {
    // 统一错误处理
    console.error('请求错误', error)
    return Promise.reject(error)
  }
)

export default request 