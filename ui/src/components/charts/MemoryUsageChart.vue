<template>
  <div class="memory-usage-chart">
    <div ref="chartRef" class="chart"></div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import * as echarts from 'echarts'
import axios from 'axios'

const chartRef = ref<HTMLDivElement | null>(null)
let chart: echarts.ECharts | null = null

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
    const response = await axios.get('/api/metrics/memory')
    const { total, used } = response.data
    const usagePercentage = (used / total * 100).toFixed(2)
    
    chart?.setOption({
      series: [{
        data: [{ value: parseFloat(usagePercentage) }]
      }]
    })
  } catch (error) {
    console.error('内存指标获取失败', error)
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
.memory-usage-chart {
  width: 100%;
  height: 300px;
}

.chart {
  width: 100%;
  height: 100%;
}
</style> 