<template>
  <div id="app">
    <div class="app-container">
      <!-- 侧边栏 -->
      <aside class="sidebar" :class="{ 'sidebar-collapsed': isSidebarCollapsed }">
        <div class="sidebar-header">
          <div class="logo-container">
            <img src="/logo.svg" alt="RustRay Logo" class="logo" />
            <span v-if="!isSidebarCollapsed" class="logo-text">RustRay</span>
          </div>
          <button class="collapse-btn" @click="toggleSidebar">
            <span class="collapse-icon">{{ isSidebarCollapsed ? '→' : '←' }}</span>
          </button>
        </div>
        
        <nav class="sidebar-nav">
          <router-link to="/" class="nav-item" :class="{ 'nav-item-collapsed': isSidebarCollapsed }">
            <span class="nav-icon">🏠</span>
            <span v-if="!isSidebarCollapsed" class="nav-text">首页</span>
          </router-link>
          
          <router-link to="/dashboard" class="nav-item" :class="{ 'nav-item-collapsed': isSidebarCollapsed }">
            <span class="nav-icon">📊</span>
            <span v-if="!isSidebarCollapsed" class="nav-text">仪表盘</span>
          </router-link>
          
          <router-link to="/tasks" class="nav-item" :class="{ 'nav-item-collapsed': isSidebarCollapsed }">
            <span class="nav-icon">📋</span>
            <span v-if="!isSidebarCollapsed" class="nav-text">任务管理</span>
          </router-link>
          
          <router-link to="/settings" class="nav-item" :class="{ 'nav-item-collapsed': isSidebarCollapsed }">
            <span class="nav-icon">⚙️</span>
            <span v-if="!isSidebarCollapsed" class="nav-text">系统设置</span>
          </router-link>
        </nav>
      </aside>

      <!-- 主内容区 -->
      <main class="main-content" :class="{ 'main-content-expanded': isSidebarCollapsed }">
        <router-view></router-view>
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const isSidebarCollapsed = ref(false)

const toggleSidebar = () => {
  isSidebarCollapsed.value = !isSidebarCollapsed.value
}
</script>

<style>
#app {
  font-family: Avenir, Helvetica, Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  color: #2c3e50;
}

.app-container {
  display: flex;
  min-height: 100vh;
}

.sidebar {
  width: 240px;
  background: #1a1a1a;
  color: #fff;
  transition: width 0.3s ease;
  display: flex;
  flex-direction: column;
}

.sidebar-collapsed {
  width: 64px;
}

.sidebar-header {
  padding: 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid #333;
}

.logo-container {
  display: flex;
  align-items: center;
  gap: 12px;
}

.logo {
  width: 32px;
  height: 32px;
}

.logo-text {
  font-size: 18px;
  font-weight: 600;
  color: #fff;
}

.collapse-btn {
  background: none;
  border: none;
  color: #fff;
  cursor: pointer;
  padding: 4px;
  border-radius: 4px;
  transition: background-color 0.2s;
}

.collapse-btn:hover {
  background: #333;
}

.collapse-icon {
  font-size: 16px;
}

.sidebar-nav {
  padding: 16px 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  color: #fff;
  text-decoration: none;
  transition: background-color 0.2s;
}

.nav-item:hover {
  background: #333;
}

.nav-item.router-link-active {
  background: #2563eb;
}

.nav-item-collapsed {
  justify-content: center;
  padding: 12px;
}

.nav-icon {
  font-size: 18px;
}

.main-content {
  flex: 1;
  transition: margin-left 0.3s ease;
  background: #f0f2f5;
  min-height: 100vh;
}

.main-content-expanded {
  margin-left: -176px;
}

@media (max-width: 768px) {
  .sidebar {
    position: fixed;
    height: 100vh;
    z-index: 1000;
    transform: translateX(0);
  }

  .sidebar-collapsed {
    transform: translateX(-240px);
  }

  .main-content {
    margin-left: 0 !important;
  }
}
</style> 