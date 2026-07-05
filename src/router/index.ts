import { createRouter, createWebHistory } from 'vue-router'
import DashboardView from '../views/DashboardView.vue'
import PlanView from '../views/PlanView.vue'
import TransactionsView from '../views/TransactionsView.vue'
import SettingsView from '../views/SettingsView.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', redirect: '/dashboard' },
    { path: '/plan', name: 'plan', component: PlanView },
    { path: '/dashboard', name: 'dashboard', component: DashboardView },
    { path: '/transactions', name: 'transactions', component: TransactionsView },
    // Analytics is lazy-loaded so the heavy ApexCharts bundle is only fetched
    // when the user first visits this route, keeping the initial load lean.
    {
      path: '/analytics',
      name: 'analytics',
      component: () => import('../views/AnalyticsView.vue'),
    },
    { path: '/settings', name: 'settings', component: SettingsView },
  ],
})

export default router
