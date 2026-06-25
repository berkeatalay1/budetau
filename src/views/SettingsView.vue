<script setup lang="ts"></script>

<template>
  <section>
    <h1 class="text-2xl font-semibold text-primary">Settings</h1>
    <p class="mt-2 text-secondary">Manage your profile, preferences, and application data.</p>

    <div class="mt-6 rounded-lg border border-border bg-card p-6 shadow-sm">
      <h2 class="text-sm font-medium uppercase tracking-wide text-secondary">Database</h2>
      <!-- TODO (Task 1D): display the resolved SQLite DB path here.
           Fetch it via a Tauri invoke (e.g. invoke<string>('get_db_path')). -->
      <p class="mt-2 text-secondary">Database path will be shown here.</p>
    </div>
  </section>
</template>
