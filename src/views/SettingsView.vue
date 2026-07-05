<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { open, save } from '@tauri-apps/plugin-dialog'
import { exportBackup, getDbPath, restoreBackup } from '../api'
import { useThemeStore } from '../stores/theme'
import { useAccountStore } from '../stores/account'
import { useToastStore } from '../stores/toast'
import type { ThemeMode } from '../stores/theme'
import type { Account } from '../types'
import AccountFormModal from '../components/settings/AccountFormModal.vue'
import EditAccountModal from '../components/settings/EditAccountModal.vue'
import ConfirmDialog from '../components/transactions/ConfirmDialog.vue'
import CategoryManagerCard from '../components/settings/CategoryManagerCard.vue'
import { formatCents } from '../lib/format'

const themeStore = useThemeStore()
const accountStore = useAccountStore()
const toast = useToastStore()

// ── DB path ───────────────────────────────────────────────────────────────────
const dbPath = ref<string | null>(null)
const isLoading = ref(false)
const dbError = ref<string | null>(null)
const copied = ref(false)

onMounted(async () => {
  isLoading.value = true
  dbError.value = null
  try {
    dbPath.value = await getDbPath()
  } catch (err) {
    dbError.value = err instanceof Error ? err.message : String(err)
  } finally {
    isLoading.value = false
  }

  if (accountStore.data.length === 0) {
    await accountStore.fetchAll()
  }
})

async function copyPath() {
  if (!dbPath.value) return
  try {
    await navigator.clipboard.writeText(dbPath.value)
    copied.value = true
    setTimeout(() => {
      copied.value = false
    }, 2000)
  } catch {
    // clipboard not available in Tauri webview — silently skip
  }
}

const themeOptions: { value: ThemeMode; label: string }[] = [
  { value: 'light', label: 'Light' },
  { value: 'dark', label: 'Dark' },
  { value: 'system', label: 'System' },
]

// ── Accounts card state ───────────────────────────────────────────────────────
const showAddModal = ref(false)
const editingAccount = ref<Account | null>(null)
const deletingAccount = ref<Account | null>(null)
const deleteError = ref<string | null>(null)
const acctActionError = ref<string | null>(null)

// Avatar color palette — cycle through via account id
const avatarColors = [
  'bg-accent/15 text-accent',
  'bg-green/15 text-green',
  'bg-amber/15 text-amber',
  'bg-red/15 text-red',
]

function avatarClass(account: Account): string {
  return avatarColors[account.id % avatarColors.length]
}

function accountInitial(account: Account): string {
  return account.name.trim().charAt(0).toUpperCase()
}

function typeLabel(t: string): string {
  return t.charAt(0).toUpperCase() + t.slice(1).toLowerCase()
}

// ── Profile id for new accounts ───────────────────────────────────────────────
function getProfileId(): number {
  return accountStore.data[0]?.profileId ?? 1
}

// ── Add ───────────────────────────────────────────────────────────────────────
async function handleAdd(payload: {
  profileId: number
  name: string
  type: string
  currency: string
  balance: number
}) {
  acctActionError.value = null
  try {
    await accountStore.add(payload)
    showAddModal.value = false
  } catch (err) {
    acctActionError.value = err instanceof Error ? err.message : String(err)
  }
}

// ── Edit ──────────────────────────────────────────────────────────────────────
async function handleEdit(payload: Account) {
  acctActionError.value = null
  try {
    await accountStore.update(payload)
    editingAccount.value = null
  } catch (err) {
    acctActionError.value = err instanceof Error ? err.message : String(err)
  }
}

// ── Open modals (methods, not inline handlers — Prettier strips the ; in
// multi-statement template expressions, which breaks the compiler) ───────────
function openAddAccount() {
  showAddModal.value = true
  acctActionError.value = null
}

function startEdit(account: Account) {
  editingAccount.value = account
  acctActionError.value = null
}

// ── Delete ────────────────────────────────────────────────────────────────────
function requestDelete(account: Account) {
  deletingAccount.value = account
  deleteError.value = null
}

async function confirmDelete() {
  if (!deletingAccount.value) return
  deleteError.value = null
  try {
    await accountStore.remove(deletingAccount.value.id)
    deletingAccount.value = null
  } catch (err) {
    // Backend blocks deletes when the account has transactions — surface it.
    deleteError.value =
      err instanceof Error
        ? err.message.includes('transaction')
          ? 'This account has transactions — reassign or delete them first.'
          : err.message
        : String(err)
    // Keep dialog open so user sees the error, then close it
    deletingAccount.value = null
  }
}

// ── Backup & restore ──────────────────────────────────────────────────────────
const backupBusy = ref(false)
const pendingRestorePath = ref<string | null>(null)
const showRestoreConfirm = ref(false)

function errMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err)
}

function defaultBackupName(): string {
  // budetau-backup-YYYY-MM-DD.db
  return `budetau-backup-${new Date().toISOString().slice(0, 10)}.db`
}

const dbFilters = [{ name: 'SQLite database', extensions: ['db'] }]

async function handleExport() {
  if (backupBusy.value) return
  backupBusy.value = true
  try {
    const path = await save({
      title: 'Export backup',
      defaultPath: defaultBackupName(),
      filters: dbFilters,
    })
    if (!path) return // user cancelled the picker
    await exportBackup(path)
    toast.show('Backup exported successfully.')
  } catch (err) {
    toast.show(`Export failed: ${errMessage(err)}`, { error: true })
  } finally {
    backupBusy.value = false
  }
}

// Pick the file first, then confirm — so the warning references a real action.
async function requestRestore() {
  if (backupBusy.value) return
  try {
    const picked = await open({
      title: 'Restore from backup',
      multiple: false,
      directory: false,
      filters: dbFilters,
    })
    if (!picked || Array.isArray(picked)) return // cancelled
    pendingRestorePath.value = picked
    showRestoreConfirm.value = true
  } catch (err) {
    toast.show(`Restore failed: ${errMessage(err)}`, { error: true })
  }
}

function reloadApp() {
  window.location.reload()
}

async function confirmRestore() {
  const path = pendingRestorePath.value
  showRestoreConfirm.value = false
  if (!path) return
  backupBusy.value = true
  try {
    await restoreBackup(path)
    // The whole database was swapped underneath us; a reload is the simplest
    // way to re-fetch every store against the restored data.
    toast.show('Database restored. Reloading…')
    setTimeout(reloadApp, 1200)
  } catch (err) {
    toast.show(`Restore failed: ${errMessage(err)}`, { error: true })
  } finally {
    backupBusy.value = false
    pendingRestorePath.value = null
  }
}

// ── Maintenance ───────────────────────────────────────────────────────────────
const recomputeBusy = ref(false)

async function handleRecomputeBalances() {
  if (recomputeBusy.value) return
  recomputeBusy.value = true
  try {
    await accountStore.recompute()
    toast.show('Account balances recalculated.')
  } catch (err) {
    toast.show(`Recalculation failed: ${errMessage(err)}`, { error: true })
  } finally {
    recomputeBusy.value = false
  }
}
</script>

<template>
  <div class="px-7 py-6 pb-8">
    <!-- Page title -->
    <div class="mb-5">
      <h1 class="text-h1 text-primary tracking-tight">Settings</h1>
    </div>

    <div class="flex max-w-[640px] flex-col gap-4">
      <!-- ── Database card ───────────────────────────────────────────────── -->
      <div class="rounded-card border border-[#eef0f3] dark:border-border bg-card p-5 shadow-card">
        <!-- Card header -->
        <div class="mb-1 flex items-center gap-2">
          <span class="text-[15px] font-semibold text-primary">Database</span>
          <span
            class="inline-flex items-center gap-[5px] rounded-pill bg-green/10 px-2 py-1 text-[11px] font-semibold text-green"
          >
            <span class="h-[6px] w-[6px] rounded-full bg-green"></span>
            On this Mac
          </span>
        </div>
        <p class="mb-4 text-[13px] leading-relaxed text-secondary">
          All your data lives in one private file. No accounts, no servers — nothing leaves your
          device.
        </p>

        <!-- File location row -->
        <div
          class="flex items-center gap-3 rounded-[9px] border border-[#eef0f3] dark:border-border bg-bg px-[14px] py-3"
        >
          <!-- DB icon -->
          <div
            class="flex h-[34px] w-[34px] flex-none items-center justify-center rounded-[8px] bg-accent/10"
          >
            <svg
              width="16"
              height="13"
              viewBox="0 0 16 13"
              fill="none"
              xmlns="http://www.w3.org/2000/svg"
            >
              <rect
                x="1"
                y="1"
                width="14"
                height="11"
                rx="2.5"
                stroke="rgb(var(--c-accent))"
                stroke-width="2"
              />
            </svg>
          </div>

          <div class="min-w-0 flex-1">
            <p class="mb-[2px] text-[12px] font-medium leading-snug text-secondary">
              File location
            </p>
            <p v-if="isLoading" class="text-[13px] text-secondary">Loading…</p>
            <p v-else-if="dbError" class="text-[13px] text-red">{{ dbError }}</p>
            <p v-else class="num break-all text-[13px] font-medium leading-snug text-primary">
              {{ dbPath }}
            </p>
          </div>

          <button
            type="button"
            class="flex-none rounded-[7px] border border-border bg-card px-3 py-2 text-[12px] font-semibold text-primary whitespace-nowrap transition-colors hover:bg-bg"
            @click="copyPath"
          >
            {{ copied ? '✓ Copied' : 'Copy path' }}
          </button>
        </div>
      </div>

      <!-- ── Appearance card ─────────────────────────────────────────────── -->
      <div class="rounded-card border border-[#eef0f3] dark:border-border bg-card p-5 shadow-card">
        <h2 class="mb-1 text-[15px] font-semibold text-primary">Appearance</h2>
        <p class="mb-4 text-[13px] leading-relaxed text-secondary">
          Choose how Budetau looks. System follows your macOS setting.
        </p>

        <!-- Segmented control -->
        <div class="inline-flex gap-[2px] rounded-[10px] bg-[#f1f3f5] dark:bg-border/20 p-[3px]">
          <button
            v-for="opt in themeOptions"
            :key="opt.value"
            type="button"
            class="rounded-[8px] px-5 py-[9px] text-[13px] font-semibold transition-all"
            :class="
              themeStore.mode === opt.value
                ? 'bg-card text-primary shadow-[0_1px_2px_rgba(16,24,40,.1)]'
                : 'text-secondary hover:text-primary'
            "
            @click="themeStore.setMode(opt.value)"
          >
            {{ opt.label }}
          </button>
        </div>
      </div>

      <!-- ── Accounts card ──────────────────────────────────────────────────── -->
      <div class="rounded-card border border-[#eef0f3] dark:border-border bg-card p-5 shadow-card">
        <!-- Card header -->
        <div class="mb-1 flex items-center justify-between">
          <h2 class="text-[15px] font-semibold text-primary">Accounts</h2>
          <button
            type="button"
            class="flex items-center gap-1.5 rounded-control border border-border bg-bg px-3 py-[7px] text-[12px] font-semibold text-primary transition-colors hover:bg-card"
            @click="openAddAccount"
          >
            <svg width="11" height="11" viewBox="0 0 11 11" fill="none" aria-hidden="true">
              <path
                d="M5.5 1v9M1 5.5h9"
                stroke="currentColor"
                stroke-width="1.75"
                stroke-linecap="round"
              />
            </svg>
            Add account
          </button>
        </div>
        <p class="mb-4 text-[13px] leading-relaxed text-secondary">
          Manage your bank accounts, cards, and wallets.
        </p>

        <!-- Inline action error (add/edit) -->
        <div
          v-if="acctActionError"
          class="mb-3 flex items-center gap-2 rounded-[9px] border border-red/40 bg-red/10 px-[13px] py-[10px]"
          role="alert"
        >
          <span class="flex-1 text-[12px] leading-relaxed text-red">{{ acctActionError }}</span>
          <button
            type="button"
            class="text-red/60 hover:text-red transition-colors"
            aria-label="Dismiss error"
            @click="acctActionError = null"
          >
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
              <path
                d="M9 3L3 9M3 3l6 6"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
              />
            </svg>
          </button>
        </div>

        <!-- Delete error banner -->
        <div
          v-if="deleteError"
          class="mb-3 flex items-center gap-2 rounded-[9px] border border-red/40 bg-red/10 px-[13px] py-[10px]"
          role="alert"
        >
          <span class="flex-1 text-[12px] leading-relaxed text-red">{{ deleteError }}</span>
          <button
            type="button"
            class="text-red/60 hover:text-red transition-colors"
            aria-label="Dismiss error"
            @click="deleteError = null"
          >
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
              <path
                d="M9 3L3 9M3 3l6 6"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
              />
            </svg>
          </button>
        </div>

        <!-- Loading state -->
        <div v-if="accountStore.isLoading" class="py-4 text-center text-[13px] text-secondary">
          Loading…
        </div>

        <!-- Empty state -->
        <div
          v-else-if="accountStore.data.length === 0"
          class="rounded-[9px] border border-dashed border-border py-8 text-center"
        >
          <p class="text-[13px] text-secondary">No accounts yet.</p>
          <button
            type="button"
            class="mt-2 text-[13px] font-semibold text-accent hover:underline"
            @click="openAddAccount"
          >
            Add your first account
          </button>
        </div>

        <!-- Account list -->
        <ul v-else class="flex flex-col divide-y divide-border/60" role="list">
          <li
            v-for="account in accountStore.data"
            :key="account.id"
            class="group flex items-center gap-3 py-[11px] first:pt-0 last:pb-0"
          >
            <!-- Avatar / initial -->
            <div
              class="flex h-9 w-9 flex-none items-center justify-center rounded-full text-[14px] font-bold"
              :class="avatarClass(account)"
              aria-hidden="true"
            >
              {{ accountInitial(account) }}
            </div>

            <!-- Name + type -->
            <div class="min-w-0 flex-1">
              <p class="truncate text-[13px] font-semibold text-primary leading-snug">
                {{ account.name }}
              </p>
              <p class="text-[11px] text-secondary">{{ typeLabel(account.type) }}</p>
            </div>

            <!-- Balance -->
            <span class="num text-[13px] font-semibold text-primary tabular-nums">
              {{ formatCents(account.balance, account.currency) }}
            </span>

            <!-- Row actions (visible on hover / focus-within) -->
            <div
              class="ml-2 flex items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100"
            >
              <button
                type="button"
                class="rounded-[6px] border border-border bg-card px-2.5 py-1.5 text-[11px] font-semibold text-secondary hover:text-primary transition-colors"
                :aria-label="`Edit ${account.name}`"
                @click="startEdit(account)"
              >
                Edit
              </button>
              <button
                type="button"
                class="rounded-[6px] border border-red/30 bg-red/5 px-2.5 py-1.5 text-[11px] font-semibold text-red hover:bg-red/10 transition-colors"
                :aria-label="`Delete ${account.name}`"
                @click="requestDelete(account)"
              >
                Delete
              </button>
            </div>
          </li>
        </ul>
      </div>

      <!-- ── Categories card ───────────────────────────────────────────── -->
      <CategoryManagerCard />

      <!-- ── Backup & restore card ──────────────────────────────────────── -->
      <div class="rounded-card border border-[#eef0f3] dark:border-border bg-card p-5 shadow-card">
        <h2 class="mb-1 text-[15px] font-semibold text-primary">Backup &amp; restore</h2>
        <p class="mb-4 text-[13px] leading-relaxed text-secondary">
          Export a timestamped copy of your database, or restore from a backup file.
        </p>

        <div class="mb-[14px] flex gap-[10px]">
          <button
            type="button"
            class="rounded-control bg-accent px-4 py-[10px] text-[13px] font-semibold text-card transition-colors hover:bg-accent/90 disabled:opacity-50 disabled:cursor-not-allowed"
            :disabled="backupBusy"
            @click="handleExport"
          >
            Export backup…
          </button>
          <button
            type="button"
            class="rounded-control border border-border bg-card px-4 py-[10px] text-[13px] font-semibold text-primary transition-colors hover:bg-bg disabled:opacity-50 disabled:cursor-not-allowed"
            :disabled="backupBusy"
            @click="requestRestore"
          >
            Restore from file…
          </button>
        </div>

        <!-- Warning banner -->
        <div
          class="flex items-start gap-[10px] rounded-[9px] border border-amber/40 bg-amber/10 px-[13px] py-[11px]"
        >
          <div
            class="flex h-[18px] w-[18px] flex-none items-center justify-center rounded-full bg-amber text-[11px] font-bold text-card"
          >
            !
          </div>
          <p class="text-[12px] leading-relaxed text-amber">
            Restoring <strong class="font-semibold">overwrites</strong> your current data. Budetau
            makes an automatic backup first.
          </p>
        </div>
      </div>

      <!-- ── Maintenance card ────────────────────────────────────────────── -->
      <div class="rounded-card border border-[#eef0f3] dark:border-border bg-card p-5 shadow-card">
        <h2 class="mb-1 text-[15px] font-semibold text-primary">Maintenance</h2>
        <p class="mb-4 text-[13px] leading-relaxed text-secondary">
          Rebuild account balances from transaction history.
        </p>

        <button
          type="button"
          class="rounded-control border border-border bg-card px-4 py-[10px] text-[13px] font-semibold text-primary transition-colors hover:bg-bg disabled:opacity-50 disabled:cursor-not-allowed"
          :disabled="recomputeBusy"
          @click="handleRecomputeBalances"
        >
          {{ recomputeBusy ? 'Recalculating…' : 'Recalculate balances' }}
        </button>
      </div>
    </div>
  </div>

  <!-- ── Modals ──────────────────────────────────────────────────────────────── -->
  <AccountFormModal
    v-if="showAddModal"
    :profile-id="getProfileId()"
    @save="handleAdd"
    @cancel="showAddModal = false"
  />

  <EditAccountModal
    v-if="editingAccount"
    :account="editingAccount"
    @save="handleEdit"
    @cancel="editingAccount = null"
  />

  <ConfirmDialog
    v-if="deletingAccount"
    title="Delete account?"
    :message="`Delete &quot;${deletingAccount.name}&quot;? This cannot be undone.`"
    confirm-label="Delete"
    :danger="true"
    @confirm="confirmDelete"
    @cancel="deletingAccount = null"
  />

  <ConfirmDialog
    v-if="showRestoreConfirm"
    title="Restore from backup?"
    message="This overwrites your current data with the selected backup. Budetau saves an automatic backup of your current database first."
    confirm-label="Restore"
    :danger="true"
    @confirm="confirmRestore"
    @cancel="showRestoreConfirm = false"
  />
</template>
