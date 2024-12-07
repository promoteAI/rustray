<template>
  <div class="storage-usage-chart">
    <div ref="chartRef" class="chart"></div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import * as echarts from 'echarts'
import axios from 'axios'

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
      title: { text: '存储使用率' },
      tooltip: {
        trigger: 'item',
        formatter: (params: any) => {
          return `${params.name}: ${formatBytes(params.value)} (${params.percent}%)`
        }
      },
      legend: {
        orient: 'vertical',
        left: 'left'
      },
      series: [
        {
          name: '存储空间',
          type: 'pie',
          radius: ['50%', '70%'],
          avoidLabelOverlap: false,
          itemStyle: {
            borderRadius: 10,
            borderColor: '#fff',
            borderWidth: 2
          },
          label: {
            show: false,
            position: 'center'
          },
          emphasis: {
            label: {
              show: true,
              fontSize: '20',
              fontWeight: 'bold'
            }
          },
          labelLine: {
            show: false
          },
          data: [
            { value: 0, name: '已用空间' },
            { value: 0, name: '可用空间' }
          ]
        }
      ]
    }
    chart.setOption(option)
  }
}

const updateChartData = async () => {
  try {
    const response = await axios.get('/api/metrics/storage')
    const { total, used, free } = response.data
    
    chart?.setOption({
      series: [{
        data: [
          { value: used, name: '已用空间' },
          { value: free, name: '可用空间' }
        ]
      }]
    })
  } catch (error) {
    console.error('存储指标获取失败', error)
  }
}

onMounted(() => {
  initChart()
  const timer = setInterval(updateChartData, 5000)
  onUnmounted(() => {
    clearInterval(timer)
    chart?.dispose()
  })
})
</script>

<style scoped>
.storage-usage-chart {
  width: 100%;
  height: 300px;
}

.chart {
  width: 100%;
  height: 100%;
}
</style> 