<template>
  <div class="page">
    <div class="page-header">
      <div>
        <h2>用户管理</h2>
        <p>查看用户信息并管理账号状态</p>
      </div>
    </div>

    <div v-if="notice" class="notice" :class="noticeType">{{ notice }}</div>

    <section class="toolbar">
      <div class="input-wrap grow">
        <Search :size="17" />
        <input v-model.trim="keyword" type="text" placeholder="搜索昵称、邮箱或学校" @keyup.enter="search" />
      </div>
      <button class="btn btn-primary" type="button" @click="search">
        <Search :size="16" />
        查询
      </button>
    </section>

    <section class="panel">
      <table class="table">
        <thead>
          <tr>
            <th>ID</th><th>昵称</th><th>邮箱</th><th>学校</th><th>专业</th><th>注册时间</th><th>身份</th><th>状态</th><th class="actions-col">操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="user in list" :key="user.id">
            <td>{{ user.id }}</td>
            <td>{{ user.nickname }}</td>
            <td>{{ user.email }}</td>
            <td>{{ user.school_name }}</td>
            <td>{{ user.major_name }}</td>
            <td>{{ formatTime(user.created_at) }}</td>
            <td><span class="badge" :class="user.is_admin ? 'info' : 'muted'">{{ user.is_admin ? '管理员' : '用户' }}</span></td>
            <td><span class="badge" :class="user.is_disabled ? 'danger' : 'success'">{{ user.is_disabled ? '已禁用' : '正常' }}</span></td>
            <td class="actions">
              <button class="btn btn-sm" type="button" :class="user.is_disabled ? 'btn-success' : 'btn-danger'" @click="toggleDisable(user)">
                {{ user.is_disabled ? '启用' : '禁用' }}
              </button>
              <button class="btn btn-sm btn-ghost" type="button" @click="toggleAdmin(user)">
                {{ user.is_admin ? '取消管理员' : '设为管理员' }}
              </button>
              <button class="btn btn-sm btn-ghost" type="button" @click="openReset(user)">
                <KeyRound :size="14" />
                重置密码
              </button>
            </td>
          </tr>
        </tbody>
      </table>
      <div v-if="!list.length" class="empty-state">暂无用户</div>
    </section>

    <PaginationBar :page="page" :total="total" :page-size="pageSize" @change="changePage" />

    <AppModal v-model="resetVisible" title="重置用户密码" width="420px" @close="resetVisible = false">
      <label class="field">
        <span>新密码（至少 6 位）</span>
        <div class="input-wrap">
          <KeyRound :size="17" />
          <input v-model="newPassword" type="password" placeholder="请输入新密码" />
        </div>
      </label>
      <template #footer>
        <button class="btn btn-ghost" type="button" @click="resetVisible = false">取消</button>
        <button class="btn btn-primary" type="button" :disabled="saving" @click="submitReset">确认重置</button>
      </template>
    </AppModal>
  </div>
</template>

<script setup>
import { onMounted, ref } from 'vue'
import { KeyRound, Search } from '@lucide/vue'
import { api } from '../api'
import AppModal from '../components/AppModal.vue'
import PaginationBar from '../components/PaginationBar.vue'

const list = ref([])
const total = ref(0)
const page = ref(1)
const pageSize = ref(20)
const keyword = ref('')
const notice = ref('')
const noticeType = ref('')
const resetVisible = ref(false)
const resetTarget = ref(null)
const newPassword = ref('')
const saving = ref(false)

function flash(message, type = 'success') {
  notice.value = message
  noticeType.value = type
  setTimeout(() => {
    notice.value = ''
  }, 2600)
}

async function load() {
  const data = await api('/admin/users', {
    params: { keyword: keyword.value, page: page.value, page_size: pageSize.value }
  })
  list.value = data.list
  total.value = data.total
}

function search() {
  page.value = 1
  load().catch((e) => flash(e.message, 'error'))
}

function changePage(next) {
  page.value = next
  load().catch((e) => flash(e.message, 'error'))
}

async function toggleDisable(user) {
  const action = user.is_disabled ? '启用' : '禁用'
  if (!window.confirm(`确定${action}用户「${user.nickname}」吗？`)) return
  try {
    await api(`/admin/users/${user.id}/status`, {
      method: 'PUT',
      body: { is_disabled: !user.is_disabled }
    })
    flash(`已${action}用户`)
    load()
  } catch (e) {
    flash(e.message, 'error')
  }
}

async function toggleAdmin(user) {
  const action = user.is_admin ? '取消' : '设置为'
  if (!window.confirm(`确定${action}「${user.nickname}」的管理员身份吗？`)) return
  try {
    await api(`/admin/users/${user.id}/admin`, {
      method: 'PUT',
      body: { is_admin: !user.is_admin }
    })
    flash(`已${action}管理员`)
    load()
  } catch (e) {
    flash(e.message, 'error')
  }
}

function openReset(user) {
  resetTarget.value = user
  newPassword.value = ''
  resetVisible.value = true
}

async function submitReset() {
  if (newPassword.value.length < 6) {
    flash('新密码至少 6 位', 'error')
    return
  }
  saving.value = true
  try {
    await api(`/admin/users/${resetTarget.value.id}/password`, {
      method: 'PUT',
      body: { new_password: newPassword.value }
    })
    resetVisible.value = false
    flash('密码已重置')
  } catch (e) {
    flash(e.message, 'error')
  } finally {
    saving.value = false
  }
}

function formatTime(value) {
  return String(value || '').replace('T', ' ').slice(0, 16)
}

onMounted(() => load().catch((e) => flash(e.message, 'error')))
</script>