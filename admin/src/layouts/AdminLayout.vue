<template>
  <div class="admin-shell">
    <aside class="sidebar" :class="{ collapsed: collapsed }">
      <div class="brand">
        <span class="brand-mark">星</span>
        <span v-if="!collapsed" class="brand-name">校考星管理端</span>
      </div>
      <nav class="nav">
        <router-link
          v-for="item in navItems"
          :key="item.to"
          :to="item.to"
          class="nav-item"
          :class="{ active: route.path.startsWith(item.to) }"
        >
          <component :is="item.icon" :size="19" />
          <span v-if="!collapsed">{{ item.label }}</span>
        </router-link>
      </nav>
    </aside>
    <div class="main">
      <header class="topbar">
        <button class="icon-btn" type="button" @click="collapsed = !collapsed">
          <Menu :size="20" />
        </button>
        <div class="topbar-title">{{ route.meta.title || '管理端' }}</div>
        <div class="topbar-right">
          <span class="admin-name">{{ admin?.nickname || '管理员' }}</span>
          <button class="btn btn-ghost" type="button" @click="logout">
            <LogOut :size="16" />
            退出
          </button>
        </div>
      </header>
      <main class="content">
        <router-view />
      </main>
    </div>
  </div>
</template>

<script setup>
import { reactive, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  BarChart3,
  FileText,
  Flag,
  LogOut,
  Menu,
  Users,
  Vote
} from '@lucide/vue'
import { clearAuth, getAdminUser } from '../api'

const route = useRoute()
const router = useRouter()
const collapsed = ref(false)
const admin = reactive(getAdminUser() || { nickname: '管理员' })

const navItems = [
  { to: '/dashboard', label: '统计看板', icon: BarChart3 },
  { to: '/users', label: '用户管理', icon: Users },
  { to: '/resources', label: '资源审核', icon: FileText },
  { to: '/votes', label: '考点审核', icon: Vote },
  { to: '/teams', label: '小队管理', icon: Flag }
]

function logout() {
  clearAuth()
  router.replace('/login')
}
</script>