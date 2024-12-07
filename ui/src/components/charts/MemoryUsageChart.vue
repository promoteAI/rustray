<template>
  <div class="memory-usage-chart">
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
      title: { text: '内存使用率' },
      tooltip: {
        trigger: 'axis',
        formatter: '{b}: {c}%'
      },
      xAxis: { 
        type: 'category',
        data: ['已用', '空闲']
      },
      yAxis: { 
        type: 'value',
        max: 100,
        axisLabel: {
          formatter: '{value}%'
        }
      },
      series: [{
        name: '内存使用',
        type: 'gauge',
        detail: { formatter: '{value}%' },
        data: [{ value: 0 }],
        min: 0,
        max: 100,
        splitNumber: 10,
        axisLine: {
          lineStyle: {
            color: [
              [0.3, '#67C23A'],
              [0.7, '#E6A23C'],
              [1, '#F56C6C']
            ],
            width: 30
          }
        }
      }]
    }
    chart.setOption(option)
  }
}

const updateChartData = async () => {
  try {
    const response = await axios.get(API_ROUTES.METRICS.MEMORY)
    const { total, used } = response.data
    const usagePercentage = (used / total * 100).toFixed(2)
    
    chart?.setOption({
      series: [{
        data: [{ 
          value: parseFloat(usagePercentage),
          name: `已用: ${formatBytes(used)} / ${formatBytes(total)}`
        }]
      }]
    })
  } catch (error) {
    console.error('内存指标获取失败:', error)
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
.memory-usage-chart {
  width: 100%;
  height: 300px;
}

.chart {
  width: 100%;
  height: 100%;
}
</style> 