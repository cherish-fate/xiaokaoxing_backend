<template>
  <div class="page">
    <div class="page-header">
      <div>
        <h2>统计看板</h2>
        <p>平台核心数据与待办审核概览</p>
      </div>
      <button class="btn btn-ghost" type="button" @click="load">
        <RefreshCw :size="16" />
        刷新
      </button>
    </div>

    <div v-if="error" class="notice error">{{ error }}</div>
    <div v-if="!data" class="empty-state">加载中...</div>

    <template v-else>
      <section class="stat-grid">
        <article v-for="card in cards" :key="card.label" class="stat-card" :class="card.tone">
          <component :is="card.icon" :size="21" />
          <div>
            <span>{{ card.label }}</span>
            <strong>{{ card.value }}</strong>
          </div>
        </article>
      </section>

      <section class="panel-grid">
        <article class="panel">
          <header class="panel-header"><h3>近 7 日打卡</h3></header>
          <div v-if="data.checkin_trend.length" class="bar-chart">
            <div v-for="item in data.checkin_trend" :key="item.date" class="bar-col">
              <span class="bar-value">{{ item.count }}</span>
              <div class="bar-track">
                <div class="bar-fill" :style="{ height: barHeight(item.count) }"></div>
              </div>
              <span class="bar-label">{{ shortDate(item.date) }}</span>
            </div>
          </div>
          <div v-else class="empty-state">暂无打卡数据</div>
        </article>

        <article class="panel">
          <header class="panel-header"><h3>资源分类分布</h3></header>
          <div v-if="data.resource_categories.length" class="category-list">
            <div v-for="item in data.resource_categories" :key="item.category" class="category-row">
              <span>{{ item.category }}</span>
              <div class="bar-track small">
                <div class="bar-fill teal" :style="{ width: categoryWidth(item.count) }"></div>
              </div>
              <strong>{{ item.count }}</strong>
            </div>
          </div>
          <div v-else class="empty-state">暂无资源</div>
        </article>
      </section>

      <section class="recent-grid">
        <article class="panel">
          <header class="panel-header"><h3>最近注册用户</h3></header>
          <table class="table">
            <thead><tr><th>昵称</th><th>邮箱</th><th>注册时间</th></tr></thead>
            <tbody>
              <tr v-for="user in data.recent_users" :key="user.id">
                <td>{{ user.nickname }}</td><td>{{ user.email }}</td><td>{{ shortTime(user.created_at) }}</td>
              </tr>
            </tbody>
          </table>
        </article>

        <article class="panel">
          <header class="panel-header"><h3>最新资源</h3></header>
          <table class="table">
            <thead><tr><th>标题</th><th>分类</th><th>状态</th></tr></thead>
            <tbody>
              <tr v-for="res in data.recent_resources" :key="res.id">
                <td>{{ res.title }}</td><td>{{ res.category }}</td><td><span class="badge" :class="statusClass(res.status)">{{ res.status }}</span></td>
              </tr>
            </tbody>
          </table>
        </article>

        <article class="panel">
          <header class="panel-header"><h3>最新考点</h3></header>
          <table class="table">
            <thead><tr><th>科目</th><th>考点</th><th>状态</th></tr></thead>
            <tbody>
              <tr v-for="vote in data.recent_votes" :key="vote.id">
                <td>{{ vote.subject }}</td><td>{{ vote.title }}</td><td><span class="badge" :class="statusClass(vote.status)">{{ vote.status }}</span></td>
              </tr>
            </tbody>
          </table>
        </article>
      </section>
    </template>
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue'
import {
  Ban,
  BookOpen,
  CalendarCheck,
  ClipboardList,
  FileText,
  Flag,
  RefreshCw,
  UserPlus,
  Users,
  Vote
} from '@lucide/vue'
import { api } from '../api'

const data = ref(null)
const error = ref('')

const cards = computed(() => {
  if (!data.value) return []
  const d = data.value
  return [
    { label: '用户总数', value: d.users.total, icon: Users, tone: 'blue' },
    { label: '今日新增', value: d.users.today_new, icon: UserPlus, tone: 'green' },
    { label: '已禁用用户', value: d.users.disabled, icon: Ban, tone: 'red' },
    { label: '待审核资源', value: d.content.resources_pending, icon: FileText, tone: 'amber' },
    { label: '待审核考点', value: d.content.votes_pending, icon: Vote, tone: 'purple' },
    { label: '学习小队', value: d.content.teams_total, icon: Flag, tone: 'teal' },
    { label: '打卡总数', value: d.content.checkins_total, icon: CalendarCheck, tone: 'blue' },
    { label: '文档总数', value: d.content.documents_total, icon: BookOpen, tone: 'green' },
    { label: '笔记总数', value: d.content.notes_total, icon: ClipboardList, tone: 'amber' }
  ]
})

async function load() {
  error.value = ''
  try {
    data.value = await api('/admin/stats/dashboard')
  } catch (e) {
    error.value = e.message
  }
}

function barHeight(count) {
  const max = Math.max(1, ...data.value.checkin_trend.map((i) => i.count))
  return Math.max(8, Math.round((count / max) * 100)) + '%'
}

function categoryWidth(count) {
  const max = Math.max(1, ...data.value.resource_categories.map((i) => i.count))
  return Math.max(8, Math.round((count / max) * 100)) + '%'
}

function shortDate(date) {
  return String(date || '').slice(5)
}

function shortTime(value) {
  return String(value || '').replace('T', ' ').slice(0, 16)
}

function statusClass(status) {
  if (status === '已上线' || status === '已通过') return 'success'
  if (status === '未通过' || status === '已拒绝') return 'danger'
  if (status === '审核中' || status === '待审核') return 'warning'
  return ''
}

onMounted(load)
</script>