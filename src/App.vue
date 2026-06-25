<script setup lang="ts"></script>

<template>
  <div class="flex h-screen bg-bg text-primary">
    <!-- Sidebar -->
    <aside class="flex w-60 flex-shrink-0 flex-col border-r border-border bg-card">
      <div class="px-6 py-5">
        <h1 class="text-lg font-semibold text-primary">Budetau</h1>
        <p class="text-xs text-secondary">Budget Planner</p>
      </div>
      <nav class="flex flex-1 flex-col gap-1 px-3">
        <router-link
          to="/dashboard"
          class="rounded-md px-3 py-2 text-sm font-medium text-secondary hover:bg-bg hover:text-primary"
          active-class="bg-bg text-accent"
        >
          Dashboard
        </router-link>
        <router-link
          to="/transactions"
          class="rounded-md px-3 py-2 text-sm font-medium text-secondary hover:bg-bg hover:text-primary"
          active-class="bg-bg text-accent"
        >
          Transactions
        </router-link>
        <router-link
          to="/settings"
          class="rounded-md px-3 py-2 text-sm font-medium text-secondary hover:bg-bg hover:text-primary"
          active-class="bg-bg text-accent"
        >
          Settings
        </router-link>
      </nav>
    </aside>

    <!-- Main content -->
    <main class="flex-1 overflow-y-auto p-8">
      <router-view />
    </main>
  </div>
</template>
