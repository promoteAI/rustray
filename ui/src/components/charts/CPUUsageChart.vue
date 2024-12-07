<template>
  <div class="cpu-usage-chart">
    <div class="chart-header">
      <div class="chart-title">
        <h4>CPU 使用率</h4>
        <div class="cpu-info">
          <span class="cpu-cores">{{ cores }}核心</span>
          <span class="cpu-avg">平均: {{ avgUsage }}%</span>
        </div>
      </div>
      <div class="chart-legend">
        <span class="legend-item">
          <span class="legend-color" style="background: #3b82f6"></span>
          实时使用率
        </span>
        <span class="legend-item">
          <span class="legend-color" style="background: #10b981"></span>
          平均使用率
        </span>
      </div>
    </div>
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
const cores = ref(0)
const avgUsage = ref('0.00')

interface CPUData {
  time: string
  usage: number
  avgUsage: number
}

const dataPoints = ref<CPUData[]>([])
const maxDataPoints = 30

const initChart = () => {
  if (chartRef.value) {
    chart = echarts.init(chartRef.value)
    const option: echarts.EChartsOption = {
      tooltip: {
        trigger: 'axis',
        axisPointer: {
          type: 'cross',
          label: {
            backgroundColor: '#6a7985'
          }
        },
        formatter: (params: any) => {
          const time = params[0].axisValue
          const usage = params[0].data.toFixed(1)
          const avgUsage = params[1].data.toFixed(1)
          return `时间：${time}<br/>
                 实时使用率：${usage}%<br/>
                 平均使用率：${avgUsage}%`
        }
      },
      grid: {
        top: 10,
        right: 20,
        bottom: 30,
        left: 50,
        containLabel: true
      },
      xAxis: {
        type: 'category',
        boundaryGap: false,
        data: [],
        axisLine: {
          lineStyle: {
            color: '#d1d5db'
          }
        },
        axisLabel: {
          color: '#6b7280',
          fontSize: 12
        }
      },
      yAxis: {
        type: 'value',
        min: 0,
        max: 100,
        splitNumber: 5,
        axisLine: {
          show: true,
          lineStyle: {
            color: '#d1d5db'
          }
        },
        axisLabel: {
          color: '#6b7280',
          fontSize: 12,
          formatter: '{value}%'
        },
        splitLine: {
          lineStyle: {
            color: '#e5e7eb'
          }
        }
      },
      series: [
        {
          name: '实时使用率',
          type: 'line',
          smooth: true,
          showSymbol: false,
          emphasis: {
            focus: 'series'
          },
          lineStyle: {
            width: 3,
            color: '#3b82f6'
          },
          areaStyle: {
            opacity: 0.1,
            color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
              {
                offset: 0,
                color: '#3b82f6'
              },
              {
                offset: 1,
                color: 'rgba(59, 130, 246, 0.1)'
              }
            ])
          },
          data: []
        },
        {
          name: '平均使用率',
          type: 'line',
          smooth: true,
          showSymbol: false,
          emphasis: {
            focus: 'series'
          },
          lineStyle: {
            width: 3,
            color: '#10b981'
          },
          data: []
        }
      ]
    }
    chart.setOption(option)
  }
}

const updateChartData = async () => {
  try {
    const response = await axios.get(API_ROUTES.METRICS.CPU)
    const { usage, cores: cpuCores } = response.data
    cores.value = cpuCores

    const now = new Date().toLocaleTimeString('zh-CN', {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit'
    })
    
    const currentUsage = usage.reduce((sum: number, val: number) => sum + val, 0) / usage.length
    
    dataPoints.value.push({
      time: now,
      usage: currentUsage,
      avgUsage: dataPoints.value.length > 0 
        ? (dataPoints.value.reduce((sum, point) => sum + point.usage, 0) + currentUsage) / (dataPoints.value.length + 1)
        : currentUsage
    })

    // 保留最近的数据点
    if (dataPoints.value.length > maxDataPoints) {
      dataPoints.value.shift()
    }

    // 更新平均使用率显示
    const totalUsage = dataPoints.value.reduce((sum, point) => sum + point.usage, 0)
    avgUsage.value = (totalUsage / dataPoints.value.length).toFixed(2)

    chart?.setOption({
      xAxis: {
        data: dataPoints.value.map(point => point.time)
      },
      series: [
        {
          data: dataPoints.value.map(point => point.usage)
        },
        {
          data: dataPoints.value.map(point => point.avgUsage)
        }
      ]
    })
  } catch (error) {
    console.error('CPU指标获取失败:', error)
    // 添加错误处理UI反馈
    cores.value = 0
    avgUsage.value = '0.00'
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
.cpu-usage-chart {
  height: 100%;
  min-height: 300px;
  display: flex;
  flex-direction: column;
  padding: 16px;
}

.chart-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.chart-title {
  display: flex;
  align-items: center;
  gap: 16px;
}

h4 {
  margin: 0;
  color: #111827;
  font-size: 16px;
  font-weight: 600;
}

.cpu-info {
  display: flex;
  gap: 12px;
}

.cpu-cores {
  padding: 2px 8px;
  background: #dbeafe;
  color: #2563eb;
  border-radius: 4px;
  font-size: 12px;
  font-weight: 500;
}

.cpu-avg {
  padding: 2px 8px;
  background: #d1fae5;
  color: #059669;
  border-radius: 4px;
  font-size: 12px;
  font-weight: 500;
}

.chart-legend {
  display: flex;
  gap: 16px;
}

.legend-item {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: #6b7280;
}

.legend-color {
  width: 12px;
  height: 12px;
  border-radius: 2px;
}

.chart {
  flex: 1;
  min-height: 200px;
}

@media (max-width: 640px) {
  .chart-header {
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }

  .chart-title {
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
  }

  .cpu-info {
    flex-direction: row;
  }
}
</style> 