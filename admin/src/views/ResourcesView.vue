<template>
  <div class="page">
    <div class="page-header">
      <div>
        <h2>资源审核</h2>
        <p>审核用户上传的备考资料</p>
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
        <input v-model.trim="keyword" type="text" placeholder="搜索标题、分类或科目" @keyup.enter="search" />
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
            <th>ID</th><th>标题</th><th>分类</th><th>科目</th><th>上传者</th><th>状态</th><th>热门</th><th>上传时间</th><th class="actions-col">操作</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="res in list" :key="res.id">
            <td>{{ res.id }}</td>
            <td><a class="link" :href="res.file_url" target="_blank" rel="noreferrer">{{ res.title }}</a></td>
            <td>{{ res.category }}</td>
            <td>{{ res.subject || '-' }}</td>
            <td>{{ res.uploader_name }}</td>
            <td><span class="badge" :class="statusClass(res.status)">{{ res.status }}</span></td>
            <td><span v-if="res.is_hot" class="badge info">热门</span><span v-else class="muted">-</span></td>
            <td>{{ formatTime(res.created_at) }}</td>
            <td class="actions">
              <button v-if="res.status !== '已上线'" class="btn btn-sm btn-success" type="button" @click="openReview(res, '已上线')">通过</button>
              <button v-if="res.status !== '未通过'" class="btn btn-sm btn-danger" type="button" @click="openReview(res, '未通过')">拒绝</button>
              <button class="btn btn-sm btn-ghost" type="button" @click="toggleHot(res)">
                {{ res.is_hot ? '取消热门' : '设为热门' }}
              </button>
              <button class="btn btn-sm btn-danger-ghost" type="button" @click="remove(res)">删除</button>
            </td>
          </tr>
        </tbody>
      </table>
      <div v-if="!list.length" class="empty-state">暂无资源</div>
    </section>

    <PaginationBar :page="page" :total="total" :page-size="pageSize" @change="changePage" />

    <AppModal v-model="reviewVisible" :title="reviewStatus === '未通过' ? '拒绝资源' : '通过资源'" width="520px" @close="reviewVisible = false">
      <div class="detail-block">
        <span>资源标题</span>
        <strong>{{ reviewTarget?.title }}</strong>
      </div>
      <label v-if="reviewStatus === '未通过'" class="field">
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
  { label: '审核中', value: '审核中' },
  { label: '已上线', value: '已上线' },
  { label: '未通过', value: '未通过' }
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
const reviewStatus = ref('已上线')
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
  const data = await api('/admin/resources', {
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

function openReview(res, status) {
  reviewTarget.value = res
  reviewStatus.value = status
  reviewReason.value = ''
  reviewVisible.value = true
}

async function submitReview() {
  if (reviewStatus.value === '未通过' && !reviewReason.value.trim()) {
    flash('请填写拒绝原因', 'error')
    return
  }
  saving.value = true
  try {
    await api(`/admin/resources/${reviewTarget.value.id}/review`, {
      method: 'PUT',
      body: {
        status: reviewStatus.value,
        reject_reason: reviewStatus.value === '未通过' ? reviewReason.value.trim() : null
      }
    })
    reviewVisible.value = false
    flash('资源审核完成')
    load()
  } catch (e) {
    flash(e.message, 'error')
  } finally {
    saving.value = false
  }
}

async function toggleHot(res) {
  try {
    await api(`/admin/resources/${res.id}/hot`, {
      method: 'PUT',
      body: { is_hot: !res.is_hot }
    })
    flash(res.is_hot ? '已取消热门' : '已设为热门')
    load()
  } catch (e) {
    flash(e.message, 'error')
  }
}

async function remove(res) {
  if (!window.confirm(`确定删除资源「${res.title}」吗？`)) return
  try {
    await api(`/admin/resources/${res.id}`, { method: 'DELETE' })
    flash('资源已删除')
    load()
  } catch (e) {
    flash(e.message, 'error')
  }
}

function formatTime(value) {
  return String(value || '').replace('T', ' ').slice(0, 16)
}

function statusClass(status) {
  if (status === '已上线') return 'success'
  if (status === '未通过') return 'danger'
  return 'warning'
}

onMounted(() => load().catch((e) => flash(e.message, 'error')))
</script>