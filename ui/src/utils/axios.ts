import axios from 'axios'
import { axiosConfig } from '../config/api'

// 创建axios实例
const instance = axios.create(axiosConfig)

// 请求拦截器
instance.interceptors.request.use(
  config => {
    // 在这里可以添加认证token等
    return config
  },
  error => {
    return Promise.reject(error)
  }
)

// 响应拦截器
instance.interceptors.response.use(
  response => {
    return response
  },
  error => {
    if (error.response) {
      // 处理HTTP错误状态
      switch (error.response.status) {
        case 401:
          console.error('未授权访问')
          break
        case 403:
          console.error('禁止访问')
          break
        case 404:
          console.error('资源不存在')
          break
        case 500:
          console.error('服务器错误')
          break
        default:
          console.error('网络错误')
      }
    } else if (error.request) {
      console.error('请求超时')
    } else {
      console.error('请求配置错误')
    }
    return Promise.reject(error)
  }
)

export default instance 