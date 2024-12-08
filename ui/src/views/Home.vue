<template>
  <div class="home">
    <header class="home-header">
      <h1>欢迎使用 RustRay</h1>
      <p class="subtitle">高性能分布式渲染系统</p>
    </header>

    <div class="feature-grid">
      <!-- 仪表盘卡片 -->
      <router-link to="/dashboard" class="feature-card">
        <div class="card-icon">📊</div>
        <div class="card-content">
          <h3>系统仪表盘</h3>
          <p>实时监控系统状态、资源使用和性能指标</p>
        </div>
      </router-link>

      <!-- 任务管理卡片 -->
      <router-link to="/tasks" class="feature-card">
        <div class="card-icon">📋</div>
        <div class="card-content">
          <h3>任务管理</h3>
          <p>创建、监控和管理渲染任务</p>
        </div>
      </router-link>

      <!-- 系统设置卡片 -->
      <router-link to="/settings" class="feature-card">
        <div class="card-icon">⚙️</div>
        <div class="card-content">
          <h3>系统设置</h3>
          <p>配置系统参数、性能选项和网络设置</p>
        </div>
      </router-link>

      <!-- 帮助文档卡片 -->
      <a href="https://github.com/yourusername/rustray" target="_blank" class="feature-card">
        <div class="card-icon">📚</div>
        <div class="card-content">
          <h3>帮助文档</h3>
          <p>查看使用指南、API文档和最佳实践</p>
        </div>
      </a>
    </div>

    <!-- 系统概览 -->
    <div class="overview-section">
      <h2>系统概览</h2>
      <div class="overview-grid">
        <SystemOverview />
      </div>
    </div>

    <!-- 快速开始 -->
    <div class="quickstart-section">
      <h2>快速开始</h2>
      <div class="steps-grid">
        <div class="step-card">
          <div class="step-number">1</div>
          <h3>配置系统</h3>
          <p>在系统设置中配置基本参数</p>
        </div>
        
        <div class="step-card">
          <div class="step-number">2</div>
          <h3>创建任务</h3>
          <p>在任务管理中创建新的渲染任务</p>
        </div>
        
        <div class="step-card">
          <div class="step-number">3</div>
          <h3>监控进度</h3>
          <p>在仪表盘中实时监控任务进度</p>
        </div>
        
        <div class="step-card">
          <div class="step-number">4</div>
          <h3>查看结果</h3>
          <p>在任务详情中查看渲染结果</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import SystemOverview from '../components/SystemOverview.vue'
</script>

<style scoped>
.home {
  padding: 24px;
  max-width: 1200px;
  margin: 0 auto;
}

.home-header {
  text-align: center;
  margin-bottom: 48px;
}

h1 {
  margin: 0;
  font-size: 36px;
  font-weight: 600;
  color: #1f2937;
}

.subtitle {
  margin: 8px 0 0 0;
  font-size: 18px;
  color: #6b7280;
}

.feature-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: 24px;
  margin-bottom: 48px;
}

.feature-card {
  display: flex;
  align-items: flex-start;
  gap: 16px;
  padding: 24px;
  background: white;
  border-radius: 8px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  transition: all 0.3s;
  text-decoration: none;
  color: inherit;
}

.feature-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
}

.card-icon {
  font-size: 24px;
  padding: 12px;
  background: #f3f4f6;
  border-radius: 12px;
}

.card-content h3 {
  margin: 0 0 8px 0;
  font-size: 18px;
  font-weight: 500;
  color: #1f2937;
}

.card-content p {
  margin: 0;
  font-size: 14px;
  color: #6b7280;
  line-height: 1.5;
}

.overview-section,
.quickstart-section {
  margin-bottom: 48px;
}

h2 {
  margin: 0 0 24px 0;
  font-size: 24px;
  font-weight: 600;
  color: #1f2937;
}

.overview-grid {
  background: white;
  border-radius: 8px;
  padding: 24px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.steps-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 24px;
}

.step-card {
  position: relative;
  padding: 24px;
  background: white;
  border-radius: 8px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.step-number {
  position: absolute;
  top: -12px;
  left: -12px;
  width: 32px;
  height: 32px;
  background: #2563eb;
  color: white;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
}

.step-card h3 {
  margin: 0 0 8px 0;
  font-size: 16px;
  font-weight: 500;
  color: #1f2937;
}

.step-card p {
  margin: 0;
  font-size: 14px;
  color: #6b7280;
}

@media (max-width: 640px) {
  .home {
    padding: 16px;
  }

  .home-header {
    margin-bottom: 32px;
  }

  h1 {
    font-size: 28px;
  }

  .subtitle {
    font-size: 16px;
  }

  .feature-grid,
  .steps-grid {
    grid-template-columns: 1fr;
    gap: 16px;
  }

  .overview-section,
  .quickstart-section {
    margin-bottom: 32px;
  }
}
</style> 