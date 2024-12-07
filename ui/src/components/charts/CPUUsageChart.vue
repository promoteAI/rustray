<template>
  <div class="cpu-usage-chart">
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
      title: { text: 'CPU使用率' },
      xAxis: { type: 'category' },
      yAxis: { type: 'value', max: 100 },
      series: [{
        type: 'line',
        data: [], // 动态更新
        smooth: true
      }]
    }
    chart.setOption(option)
  }
}

const updateChartData = async () => {
  try {
    const response = await axios.get('/api/metrics/cpu')
    // 更新图表数据
    chart?.setOption({
      series: [{
        data: response.data
      }]
    })
  } catch (error) {
    console.error('CPU指标获取失败', error)
  }
}

onMounted(() => {
  initChart()
  const timer = setInterval(updateChartData, 5000)
  onUnmounted(() => clearInterval(timer))
})
</script> 