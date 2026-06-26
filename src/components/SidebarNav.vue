<template>
  <!--
    Sidebar: full width (228 px) at ≥ 860 px; collapses to 60 px icon rail below.
    Uses Tailwind min-[860px] arbitrary breakpoint — no JS needed.
  -->
  <aside
    class="flex flex-shrink-0 flex-col border-r border-border bg-card transition-[width] duration-200 w-[60px] min-[860px]:w-[228px]"
    style="min-height: 0"
  >
    <!-- Wordmark -->
    <div class="flex items-center gap-2.5 px-3.5 pb-[22px] pt-[18px]">
      <!-- Logo mark -->
      <div class="relative h-7 w-7 flex-shrink-0 rounded-control bg-accent" aria-hidden="true">
        <!-- Circle-in-square motif from design -->
        <div
          class="absolute rounded-full border-[2.5px] border-white"
          style="width: 12px; height: 12px; left: 8px; top: 8px"
        ></div>
      </div>
      <!-- Label — hidden on narrow rail, visible at 860px+ -->
      <span
        class="hidden min-[860px]:block text-[16px] font-bold leading-none tracking-tight text-primary"
        >Budetau</span
      >
    </div>

    <!-- Nav items -->
    <nav class="flex flex-col gap-[3px] px-[7px] min-[860px]:px-3.5">
      <!-- Dashboard -->
      <router-link
        to="/dashboard"
        class="flex items-center gap-[11px] rounded-control px-2.5 py-[9px] text-[13px] font-medium text-secondary hover:bg-accent/10 hover:text-accent transition-colors"
        active-class="bg-accent/10 !text-accent font-semibold"
        title="Dashboard"
      >
        <!-- 2×2 grid icon -->
        <svg
          width="16"
          height="16"
          viewBox="0 0 16 16"
          fill="currentColor"
          class="flex-shrink-0"
          aria-hidden="true"
        >
          <rect x="0" y="0" width="7" height="7" rx="1.5" />
          <rect x="9" y="0" width="7" height="7" rx="1.5" />
          <rect x="0" y="9" width="7" height="7" rx="1.5" />
          <rect x="9" y="9" width="7" height="7" rx="1.5" />
        </svg>
        <span class="hidden min-[860px]:block">Dashboard</span>
      </router-link>

      <!-- Transactions -->
      <router-link
        to="/transactions"
        class="flex items-center gap-[11px] rounded-control px-2.5 py-[9px] text-[13px] font-medium text-secondary hover:bg-accent/10 hover:text-accent transition-colors"
        active-class="bg-accent/10 !text-accent font-semibold"
        title="Transactions"
      >
        <!-- Horizontal lines icon -->
        <svg
          width="16"
          height="16"
          viewBox="0 0 16 16"
          fill="currentColor"
          class="flex-shrink-0"
          aria-hidden="true"
        >
          <rect x="0" y="2" width="16" height="2" rx="1" />
          <rect x="0" y="7" width="12" height="2" rx="1" />
          <rect x="0" y="12" width="14.4" height="2" rx="1" />
        </svg>
        <span class="hidden min-[860px]:block">Transactions</span>
      </router-link>

      <!-- Analytics -->
      <router-link
        to="/analytics"
        class="flex items-center gap-[11px] rounded-control px-2.5 py-[9px] text-[13px] font-medium text-secondary hover:bg-accent/10 hover:text-accent transition-colors"
        active-class="bg-accent/10 !text-accent font-semibold"
        title="Analytics"
      >
        <!-- Bar chart icon -->
        <svg
          width="16"
          height="16"
          viewBox="0 0 16 16"
          fill="currentColor"
          class="flex-shrink-0"
          aria-hidden="true"
        >
          <rect x="0" y="9" width="4" height="7" rx="1" />
          <rect x="6" y="5" width="4" height="11" rx="1" />
          <rect x="12" y="1" width="4" height="15" rx="1" />
        </svg>
        <span class="hidden min-[860px]:block">Analytics</span>
      </router-link>

      <!-- Settings -->
      <router-link
        to="/settings"
        class="flex items-center gap-[11px] rounded-control px-2.5 py-[9px] text-[13px] font-medium text-secondary hover:bg-accent/10 hover:text-accent transition-colors"
        active-class="bg-accent/10 !text-accent font-semibold"
        title="Settings"
      >
        <!-- Circle-with-dot icon -->
        <svg
          width="16"
          height="16"
          viewBox="0 0 16 16"
          fill="none"
          class="flex-shrink-0"
          aria-hidden="true"
        >
          <circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="2" />
          <circle cx="8" cy="8" r="2.5" fill="currentColor" />
        </svg>
        <span class="hidden min-[860px]:block">Settings</span>
      </router-link>
    </nav>

    <!-- Spacer -->
    <div class="flex-1"></div>

    <!-- Profile footer -->
    <div class="border-t border-border px-[7px] pb-4 pt-3.5 min-[860px]:px-3.5">
      <div class="flex items-center gap-2.5">
        <!-- Avatar -->
        <div
          class="flex h-[30px] w-[30px] flex-shrink-0 items-center justify-center rounded-full bg-accent/20 text-[12px] font-semibold text-accent"
          aria-hidden="true"
        >
          P
        </div>
        <!-- Labels — hidden on narrow rail -->
        <div class="hidden flex-1 min-w-0 min-[860px]:block">
          <div class="text-[12px] font-semibold leading-[1.3] text-primary">Personal</div>
          <div class="text-[11px] leading-[1.3] text-secondary">Local file</div>
        </div>
      </div>
    </div>
  </aside>
</template>
