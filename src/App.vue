<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { useThemeStore } from './stores/theme'
import { useQuickAddStore } from './stores/quickAdd'
import SidebarNav from './components/SidebarNav.vue'
import AppFab from './components/AppFab.vue'
import QuickAddModal from './components/QuickAddModal.vue'
import ToastHost from './components/ToastHost.vue'

// Instantiate theme store so the watcher runs and `.dark` is applied on load.
useThemeStore()

const quickAdd = useQuickAddStore()

// Global ⌘K / Ctrl+K shortcut
function handleKeydown(e: KeyboardEvent): void {
  if (e.key === 'k' && (e.metaKey || e.ctrlKey)) {
    e.preventDefault()
    quickAdd.open()
  }
}

onMounted(() => window.addEventListener('keydown', handleKeydown))
onUnmounted(() => window.removeEventListener('keydown', handleKeydown))
</script>

<template>
  <!--
    Full-screen app shell.
    Position: relative so AppFab (position:absolute) anchors to this container.
  -->
  <div class="relative flex h-screen flex-col overflow-hidden bg-bg text-primary font-sans">
    <!-- Body: sidebar + scrollable main (the native OS title bar is the window chrome) -->
    <div class="flex flex-1 overflow-hidden">
      <SidebarNav />

      <!-- Main scrollable area -->
      <main class="flex-1 overflow-y-auto bg-bg">
        <router-view />
      </main>
    </div>

    <!-- Floating action button -->
    <AppFab />

    <!-- Quick-add modal — mounted once, opened via store -->
    <QuickAddModal />

    <!-- Global toast host — mounted once, driven by the toast store -->
    <ToastHost />
  </div>
</template>
