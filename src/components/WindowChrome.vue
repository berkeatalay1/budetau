<template>
  <!-- macOS-style title bar: traffic lights left, centered title, spacer right -->
  <div class="flex h-10 flex-shrink-0 items-center gap-2 border-b border-border bg-card px-3.5">
    <!-- Traffic light dots — decorative chrome, fixed colours per spec -->
    <div class="flex items-center gap-2">
      <div class="h-3 w-3 rounded-full" style="background: #ff5f57"></div>
      <div class="h-3 w-3 rounded-full" style="background: #febc2e"></div>
      <div class="h-3 w-3 rounded-full" style="background: #28c840"></div>
    </div>

    <!-- Centered title -->
    <span class="flex-1 text-center text-caption text-secondary">Budetau — budget.db</span>

    <!-- Balancing spacer so text stays centred -->
    <div class="w-[54px] flex-shrink-0"></div>
  </div>
</template>
