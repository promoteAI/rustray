<template>
  <div class="system-overview">
    <div class="overview-card">
      <h2>系统状态</h2>
      <div class="stats">
        <div>
          <span>节点数</span>
          <strong>{{ nodeCount }}</strong>
        </div>
        <div>
          <span>运行任务</span>
          <strong>{{ runningTasks }}</strong>
        </div>
        <div>
          <span>系统负载</span>
          <strong>{{ systemLoad }}</strong>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import axios from 'axios'

const nodeCount = ref(0)
const runningTasks = ref(0)
const systemLoad = ref('0%')

const fetchSystemOverview = async () => {
  try {
    const response = await axios.get('/api/system/overview')
    const data = response.data
    nodeCount.value = data.nodeCount
    runningTasks.value = data.runningTasks
    systemLoad.value = data.systemLoad
  } catch (error) {
    console.error('获取系统概览失败', error)
  }
}

onMounted(() => {
  fetchSystemOverview()
  // 定期刷新
  setInterval(fetchSystemOverview, 10000)
})
</script> 