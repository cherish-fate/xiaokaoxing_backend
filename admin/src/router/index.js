import { createRouter, createWebHashHistory } from 'vue-router'
import { getToken } from '../api'
import AdminLayout from '../layouts/AdminLayout.vue'
import LoginView from '../views/LoginView.vue'
import DashboardView from '../views/DashboardView.vue'
import UsersView from '../views/UsersView.vue'
import ResourcesView from '../views/ResourcesView.vue'
import VotesView from '../views/VotesView.vue'
import TeamsView from '../views/TeamsView.vue'

const routes = [
  { path: '/login', component: LoginView },
  {
    path: '/',
    component: AdminLayout,
    children: [
      { path: '', redirect: '/dashboard' },
      { path: 'dashboard', component: DashboardView, meta: { title: '统计看板' } },
      { path: 'users', component: UsersView, meta: { title: '用户管理' } },
      { path: 'resources', component: ResourcesView, meta: { title: '资源审核' } },
      { path: 'votes', component: VotesView, meta: { title: '考点审核' } },
      { path: 'teams', component: TeamsView, meta: { title: '小队管理' } }
    ]
  }
]

const router = createRouter({
  history: createWebHashHistory(),
  routes
})

router.beforeEach((to) => {
  if (to.path !== '/login' && !getToken()) {
    return '/login'
  }
  if (to.path === '/login' && getToken()) {
    return '/dashboard'
  }
})

export default router