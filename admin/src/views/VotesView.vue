<template>
  <div class="page">
    <div class="page-header">
      <div>
        <h2>考点审核</h2>
        <p>审核用户提交的考点与投票内容</p>
      </div>
    </div>

    <div v-if="notice" class="notice" :class="noticeType">{{ notice }}</div>

    <section class="toolbar">
      <div class="segmented">
        <button
          v-for="item in filters"
          :key="item.value"
          class="segment"
          :class="{ active: statusFilter === item.value }"
          type="button"
          @click="setStatus(item.value)"
        >
          {{ item.label }}
        </button>
      </div>
      <div class="input-wrap grow">
        <Search :size="17" />
        <input v-model.trim="keyword" type="text" placeholder="搜索考点名称或科目" @keyup.enter="search" />
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
            <th>ID</th><th>科目</th><th>考点名称</th><th>说明</th><th>提交者</th><th>票数</th><th>状态</th><th>提交时间</th><th class="actions-col">操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="vote in list" :key="vote.id">
            <td>{{ vote.id }}</td>
            <td>{{ vote.subject }}</td>
            <td>{{ vote.title }}</td>
            <td>{{ vote.description || '-' }}</td>
            <td>{{ vote.submitter_name }}</td>
            <td>{{ vote.vote_count }}</td>
            <td><span class="badge" :class="statusClass(vote.status)">{{ vote.status }}</span></td>
            <td>{{ formatTime(vote.created_at) }}</td>
            <td class="actions">
              <button v-if="vote.status !== '已通过'" class="btn btn-sm btn-success" type="button" @click="openReview(vote, '已通过')">通过</button>
              <button v-if="vote.status !== '已拒绝'" class="btn btn-sm btn-danger" type="button" @click="openReview(vote, '已拒绝')">拒绝</button>
              <button class="btn btn-sm btn-danger-ghost" type="button" @click="remove(vote)">删除</button>
            </td>
          </tr>
        </tbody>
      </table>
      <div v-if="!list.length" class="empty-state">暂无考点</div>
    </section>

    <PaginationBar :page="page" :total="total" :page-size="pageSize" @change="changePage" />

    <AppModal v-model="reviewVisible" :title="reviewStatus === '已拒绝' ? '拒绝考点' : '通过考点'" width="520px" @close="reviewVisible = false">
      <div class="detail-block">
        <span>考点名称</span>
        <strong>{{ reviewTarget?.title }}</strong>
      </div>
      <label v-if="reviewStatus === '已拒绝'" class="field">
        <span>拒绝原因</span>
        <textarea v-model="reviewReason" rows="4" placeholder="请填写拒绝原因" />
      </label>
      <template #footer>
        <button class="btn btn-ghost" type="button" @click="reviewVisible = false">取消</button>
        <button class="btn btn-primary" type="button" :disabled="saving" @click="submitReview">确认审核</button>
      </template>
    </AppModal>
  </div>
</template>

<script setup>
import { onMounted, ref } from 'vue'
import { Search } from '@lucide/vue'
import { api } from '../api'
import AppModal from '../components/AppModal.vue'
import PaginationBar from '../components/PaginationBar.vue'

const filters = [
  { label: '全部', value: '' },
  { label: '待审核', value: '待审核' },
  { label: '已通过', value: '已通过' },
  { label: '已拒绝', value: '已拒绝' }
]

const list = ref([])
const total = ref(0)
const page = ref(1)
const pageSize = ref(20)
const keyword = ref('')
const statusFilter = ref('')
const notice = ref('')
const noticeType = ref('')
const reviewVisible = ref(false)
const reviewTarget = ref(null)
const reviewStatus = ref('已通过')
const reviewReason = ref('')
const saving = ref(false)

function flash(message, type = 'success') {
  notice.value = message
  noticeType.value = type
  setTimeout(() => {
    notice.value = ''
  }, 2600)
}

async function load() {
  const data = await api('/admin/votes', {
    params: { status: statusFilter.value, keyword: keyword.value, page: page.value, page_size: pageSize.value }
  })
  list.value = data.list
  total.value = data.total
}

function setStatus(value) {
  statusFilter.value = value
  page.value = 1
  load().catch((e) => flash(e.message, 'error'))
}

function search() {
  page.value = 1
  load().catch((e) => flash(e.message, 'error'))
}

function changePage(next) {
  page.value = next
  load().catch((e) => flash(e.message, 'error'))
}

function openReview(vote, status) {
  reviewTarget.value = vote
  reviewStatus.value = status
  reviewReason.value = ''
  reviewVisible.value = true
}

async function submitReview() {
  if (reviewStatus.value === '已拒绝' && !reviewReason.value.trim()) {
    flash('请填写拒绝原因', 'error')
    return
  }
  saving.value = true
  try {
    await api(`/admin/votes/${reviewTarget.value.id}/review`, {
      method: 'PUT',
      body: {
        status: reviewStatus.value,
        reject_reason: reviewStatus.value === '已拒绝' ? reviewReason.value.trim() : null
      }
    })
    reviewVisible.value = false
    flash('考点审核完成')
    load()
  } catch (e) {
    flash(e.message, 'error')
  } finally {
    saving.value = false
  }
}

async function remove(vote) {
  if (!window.confirm(`确定删除考点「${vote.title}」吗？`)) return
  try {
    await api(`/admin/votes/${vote.id}`, { method: 'DELETE' })
    flash('考点已删除')
    load()
  } catch (e) {
    flash(e.message, 'error')
  }
}

function formatTime(value) {
  return String(value || '').replace('T', ' ').slice(0, 16)
}

function statusClass(status) {
  if (status === '已通过') return 'success'
  if (status === '已拒绝') return 'danger'
  return 'warning'
}

onMounted(() => load().catch((e) => flash(e.message, 'error')))
</script>