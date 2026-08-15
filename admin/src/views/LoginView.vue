<template>
  <div class="login-page">
    <div class="login-panel">
      <div class="login-brand"><span>星</span><h1>校考星管理端</h1></div>
      <form class="login-form" @submit.prevent="submit">
        <label class="field">
          <span>邮箱</span>
          <div class="input-wrap">
            <Mail :size="17" />
            <input v-model.trim="email" type="email" placeholder="admin@xiaokaoxing.com" autocomplete="username" required />
          </div>
        </label>
        <label class="field">
          <span>密码</span>
          <div class="input-wrap">
            <Lock :size="17" />
            <input v-model="password" type="password" placeholder="请输入密码" autocomplete="current-password" required />
          </div>
        </label>
        <p v-if="error" class="form-error">{{ error }}</p>
        <button class="btn btn-primary btn-block" type="submit" :disabled="loading">
          <ArrowRight :size="17" />
          {{ loading ? '登录中...' : '登录' }}
        </button>
      </form>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { ArrowRight, Lock, Mail } from '@lucide/vue'
import { api, setAdminUser, setToken } from '../api'

const router = useRouter()
const email = ref('admin@xiaokaoxing.com')
const password = ref('')
const loading = ref(false)
const error = ref('')

async function submit() {
  loading.value = true
  error.value = ''
  try {
    const data = await api('/admin/login', {
      method: 'POST',
      body: { email: email.value, password: password.value },
      auth: false
    })
    setToken(data.token)
    setAdminUser({ id: data.id, nickname: data.nickname, email: data.email })
    router.replace('/dashboard')
  } catch (e) {
    error.value = e.message || '登录失败'
  } finally {
    loading.value = false
  }
}
</script>