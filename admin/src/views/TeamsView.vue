<template>
  <div class="page">
    <div class="page-header">
      <div>
        <h2>小队管理</h2>
        <p>查看学习小队并解散违规小队</p>
      </div>
    </div>

    <div v-if="notice" class="notice" :class="noticeType">{{ notice }}</div>

    <section class="toolbar">
      <div class="input-wrap grow">
        <Search :size="17" />
        <input v-model.trim="keyword" type="text" placeholder="搜索小队名称或科目" @keyup.enter="search" />
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
            <th>ID</th><th>小队名称</th><th>科目</th><th>简介</th><th>队长</th><th>成员</th><th>上限</th><th>创建时间</th><th class="actions-col">操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="team in list" :key="team.id">
            <td>{{ team.id }}</td>
            <td>{{ team.name }}</td>
            <td>{{ team.subject }}</td>
            <td>{{ team.description || '-' }}</td>
            <td>{{ team.creator_name }}</td>
            <td>{{ team.member_count }}</td>
            <td>{{ team.max_members }}</td>
            <td>{{ formatTime(team.created_at) }}</td>
            <td class="actions">
              <button class="btn btn-sm btn-danger" type="button" @click="remove(team)">解散</button>
            </td>
          </tr>
        </tbody>
      </table>
      <div v-if="!list.length" class="empty-state">暂无小队</div>
    </section>

    <PaginationBar :page="page" :total="total" :page-size="pageSize" @change="changePage" />
  </div>
</template>

<script setup>
import { onMounted, ref } from 'vue'
import { Search } from '@lucide/vue'
import { api } from '../api'
import PaginationBar from '../components/PaginationBar.vue'

const list = ref([])
const total = ref(0)
const page = ref(1)
const pageSize = ref(20)
const keyword = ref('')
const notice = ref('')
const noticeType = ref('')

function flash(message, type = 'success') {
  notice.value = message
  noticeType.value = type
  setTimeout(() => {
    notice.value = ''
  }, 2600)
}

async function load() {
  const data = await api('/admin/teams', {
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

async function remove(team) {
  if (!window.confirm(`确定解散小队「${team.name}」吗？`)) return
  try {
    await api(`/admin/teams/${team.id}`, { method: 'DELETE' })
    flash('小队已解散')
    load()
  } catch (e) {
    flash(e.message, 'error')
  }
}

function formatTime(value) {
  return String(value || '').replace('T', ' ').slice(0, 16)
}

onMounted(() => load().catch((e) => flash(e.message, 'error')))
</script>