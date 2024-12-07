<template>
  <div class="network-usage-chart">
    <div ref="chartRef" class="chart"></div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import * as echarts from 'echarts'
import axios from '../../utils/axios'
import { API_ROUTES } from '../../config/api'

const chartRef = ref<HTMLDivElement | null>(null)
let chart: echarts.ECharts | null = null

interface NetworkData {
  time: string
  sent: number
  received: number
}

const dataPoints = ref<NetworkData[]>([])

const formatBytes = (bytes: number): string => {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

const initChart = () => {
  if (chartRef.value) {
    chart = echarts.init(chartRef.value)
    const option: echarts.EChartsOption = {
      title: { text: '网络使用率' },
      tooltip: {
        trigger: 'axis',
        formatter: (params: any) => {
          const time = params[0].axisValue
          const sent = formatBytes(params[0].data)
          const received = formatBytes(params[1].data)
          return `时间：${time}<br/>发送：${sent}<br/>接收：${received}`
        }
      },
      legend: {
        data: ['发送', '接收']
      },
      xAxis: {
        type: 'category',
        data: []
      },
      yAxis: {
        type: 'value',
        axisLabel: {
          formatter: (value: number) => formatBytes(value)
        }
      },
      series: [
        {
          name: '发送',
          type: 'line',
          data: [],
          smooth: true,
          areaStyle: {
            opacity: 0.1
          }
        },
        {
          name: '接收',
          type: 'line',
          data: [],
          smooth: true,
          areaStyle: {
            opacity: 0.1
          }
        }
      ]
    }
    chart.setOption(option)
  }
}

const updateChartData = async () => {
  try {
    const response = await axios.get(API_ROUTES.METRICS.NETWORK)
    const { bytes_sent, bytes_recv } = response.data
    
    const now = new Date().toLocaleTimeString('zh-CN', {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit'
    })
    
    dataPoints.value.push({
      time: now,
      sent: bytes_sent,
      received: bytes_recv
    })

    // 保留最近10个数据点
    if (dataPoints.value.length > 10) {
      dataPoints.value.shift()
    }

    chart?.setOption({
      xAxis: {
        data: dataPoints.value.map(point => point.time)
      },
      series: [
        {
          data: dataPoints.value.map(point => point.sent)
        },
        {
          data: dataPoints.value.map(point => point.received)
        }
      ]
    })
  } catch (error) {
    console.error('网络指标获取失败:', error)
  }
}

// 监听窗口大小变化
const handleResize = () => {
  chart?.resize()
}

// 提供刷新方法给父组件
const refresh = () => {
  return updateChartData()
}

window.addEventListener('resize', handleResize)

onMounted(() => {
  initChart()
  updateChartData()
  const timer = setInterval(updateChartData, 5000)
  
  onUnmounted(() => {
    clearInterval(timer)
    window.removeEventListener('resize', handleResize)
    chart?.dispose()
  })
})

defineExpose({
  refresh
})
</script>

<style scoped>
.network-usage-chart {
  width: 100%;
  height: 300px;
}

.chart {
  width: 100%;
  height: 100%;
}
</style> 