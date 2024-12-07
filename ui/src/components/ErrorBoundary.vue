<template>
  <div>
    <slot v-if="!error"></slot>
    <div v-else class="error-container">
      <h2>发生错误</h2>
      <p>{{ error.message }}</p>
      <button @click="resetError">重试</button>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue'

const error = ref(null)

const resetError = () => {
  error.value = null
}
</script> 