<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, reactive, ref } from 'vue'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import { bills, categories, cents, clone, emptyData, planFor, summarize, validateData, validateEntry } from './lib/basicBudget'
import type { BudgetData, Entry } from './lib/basicBudget'

const native = isTauri()
const today = new Intl.DateTimeFormat('en-CA', { timeZone: 'America/New_York', year: 'numeric', month: '2-digit', day: '2-digit' }).format(new Date())
const month = ref(today.slice(0, 7))
const data = ref<BudgetData>(emptyData())
const ready = ref(false), busy = ref(false), error = ref(''), message = ref('')
const summary = computed(() => summarize(data.value, month.value))
const money = (amount: number) => new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format(amount / 100)
const monthLabel = computed(() => new Date(month.value + '-01T12:00:00').toLocaleDateString('en-US', { month: 'long', year: 'numeric' }))
const monthOptions = computed(() => {
  const year = Number(month.value.slice(0, 4))
  const available = new Set(Object.keys(data.value.months))
  for (let y = Math.max(1900, year - 1); y <= Math.min(2200, year + 1); y++) {
    for (let m = 1; m <= 12; m++) available.add(`${y}-${String(m).padStart(2, '0')}`)
  }
  return [...available].sort().map(value => ({ value, label: new Date(value + '-01T12:00:00').toLocaleDateString('en-US', { month: 'long', year: 'numeric' }) }))
})
const entries = computed(() => [...summary.value.entries].sort((a, b) => b.date.localeCompare(a.date)))
const entryGroups = computed(() => {
  const groups: { date: string; entries: Entry[] }[] = []
  for (const entry of entries.value) {
    const last = groups[groups.length - 1]
    if (last?.date === entry.date) last.entries.push(entry)
    else groups.push({ date: entry.date, entries: [entry] })
  }
  return groups
})
const form = reactive({ amount: '', category: 'groceries', kind: 'purchase' as Entry['kind'], date: today, note: '', id: '', hasExtra: false, extraAmount: '', extraNote: '' })
const pendingDelete = ref('')
const selectedId = ref('')
const selectedEntry = computed(() => data.value.entries.find(e => e.id === selectedId.value))
const lastDeleted = ref<{ entry: Entry; index: number } | null>(null)
const entryDialog = ref<HTMLDialogElement | null>(null), detailsDialog = ref<HTMLDialogElement | null>(null), entryOptions = ref<HTMLDetailsElement | null>(null)
const settings = ref<HTMLDialogElement | null>(null), restoreDialog = ref<HTMLDialogElement | null>(null), fileInput = ref<HTMLInputElement | null>(null), amountInput = ref<HTMLInputElement | null>(null)
const pendingRestore = ref<BudgetData | null>(null)
const draft = reactive({ pay: '', paychecks: 2, extra: '', savings: '', limits: {} as Record<string, string>, bills: {} as Record<string, string> })
const storageKey = 'budetau-basic-v1'
let messageTimer: ReturnType<typeof setTimeout> | undefined
function showMessage(text: string) {
  if (messageTimer) clearTimeout(messageTimer)
  message.value = text
  messageTimer = setTimeout(() => { message.value = '' }, 4500)
}
onUnmounted(() => { if (messageTimer) clearTimeout(messageTimer) })

async function commit(next: BudgetData, success: string) {
  if (!ready.value || busy.value) return false
  busy.value = true; error.value = ''; message.value = ''
  try {
    validateData(next)
    const payload = JSON.stringify(next)
    if (native) await invoke('save_basic_budget', { payload })
    else localStorage.setItem(storageKey, payload)
    data.value = next
    showMessage(success)
    return true
  } catch (e) { error.value = 'Could not save. Your entry has been kept. ' + String(e); return false }
  finally { busy.value = false }
}
onMounted(async () => {
  try {
    const payload = native ? await invoke<string | null>('load_basic_budget') : localStorage.getItem(storageKey)
    const parsed = payload ? JSON.parse(payload) : null
    data.value = parsed === null ? emptyData() : validateData(parsed)
    ready.value = true
  } catch (e) { error.value = 'Could not load your budget. No data has been changed. ' + String(e) }
})
function changeMonth() {
  if (!/^\d{4}-(0[1-9]|1[0-2])$/.test(month.value)) month.value = today.slice(0, 7)
  cancelEdit(); entryDialog.value?.close(); detailsDialog.value?.close(); message.value = ''
  form.date = month.value === today.slice(0, 7) ? today : month.value + '-01'
}
function shiftMonth(offset: number) {
  const next = new Date(`${month.value}-01T12:00:00Z`)
  next.setUTCMonth(next.getUTCMonth() + offset)
  month.value = next.toISOString().slice(0, 7)
  changeMonth()
}
function clearExtra() { form.hasExtra = false; form.extraAmount = ''; form.extraNote = '' }
function changeKind() { if (form.kind === 'payment') { form.category = ''; clearExtra() } else if (!form.category) form.category = 'groceries' }
function changeExtra() { if (form.hasExtra) form.extraAmount = form.amount; else clearExtra() }
async function addEntry() {
  if (busy.value) return
  try {
    const entry: Entry = { id: form.id || crypto.randomUUID(), date: form.date, amount: cents(form.amount), category: form.kind === 'payment' ? '' : form.category, kind: form.kind, note: form.note.trim() }
    if (form.hasExtra && form.kind !== 'payment') entry.extra = { amount: cents(form.extraAmount), note: form.extraNote.trim() }
    validateEntry(entry)
    const next = clone(data.value)
    next.months[entry.date.slice(0, 7)] ??= planFor(next, entry.date.slice(0, 7))
    const index = next.entries.findIndex(e => e.id === entry.id)
    if (index >= 0) next.entries[index] = entry; else next.entries.push(entry)
    if (await commit(next, index >= 0 ? 'Transaction updated.' : 'Expense saved.')) { month.value = entry.date.slice(0, 7); entryDialog.value?.close(); cancelEdit() }
  } catch (e) { error.value = String(e instanceof Error ? e.message : e) }
}
function cancelEdit() { form.id = ''; form.amount = ''; form.note = ''; form.kind = 'purchase'; clearExtra(); if (!form.category) form.category = 'groceries' }
async function openEntry() {
  error.value = ''; cancelEdit()
  form.date = month.value === today.slice(0, 7) ? today : month.value + '-01'
  if (entryOptions.value) entryOptions.value.open = false
  await nextTick(); entryDialog.value?.showModal(); amountInput.value?.focus()
}
async function editEntry(entry: Entry) {
  error.value = ''; detailsDialog.value?.close()
  Object.assign(form, { id: entry.id, date: entry.date, category: entry.category, kind: entry.kind, note: entry.note, amount: (entry.amount / 100).toFixed(2), hasExtra: Boolean(entry.extra), extraAmount: entry.extra ? (entry.extra.amount / 100).toFixed(2) : '', extraNote: entry.extra?.note ?? '' })
  if (entryOptions.value) entryOptions.value.open = entry.kind !== 'purchase' || Boolean(entry.extra)
  await nextTick(); entryDialog.value?.showModal(); amountInput.value?.focus()
}
async function showDetails(entry: Entry) {
  selectedId.value = entry.id; pendingDelete.value = ''; error.value = ''
  await nextTick(); detailsDialog.value?.showModal()
}
async function removeEntry(id: string) {
  if (pendingDelete.value !== id) { pendingDelete.value = id; return }
  const index = data.value.entries.findIndex(e => e.id === id)
  if (index < 0) return
  const entry = clone(data.value.entries[index])
  const next = clone(data.value); next.entries.splice(index, 1)
  if (await commit(next, 'Transaction deleted.')) { lastDeleted.value = { entry, index }; pendingDelete.value = ''; detailsDialog.value?.close() }
}
async function undoDelete() {
  if (!lastDeleted.value || busy.value) return
  const { entry, index } = lastDeleted.value
  const next = clone(data.value)
  if (next.entries.some(e => e.id === entry.id)) return
  next.entries.splice(Math.min(index, next.entries.length), 0, clone(entry))
  if (await commit(next, 'Transaction restored.')) lastDeleted.value = null
}
function openSettings() {
  error.value = ''
  const p = summary.value.plan
  Object.assign(draft, { pay: (p.pay / 100).toFixed(2), paychecks: p.paychecks, extra: (p.extra / 100).toFixed(2), savings: (p.savings / 100).toFixed(2), limits: Object.fromEntries(categories.map(c => [c.id, (p.limits[c.id] / 100).toFixed(2)])), bills: Object.fromEntries(bills.map(b => [b.id, (p.bills[b.id] / 100).toFixed(2)])) })
  settings.value?.showModal()
}
async function saveSettings() {
  try {
    const next = clone(data.value)
    next.months[month.value] = { pay: cents(draft.pay), paychecks: Number(draft.paychecks), extra: cents(draft.extra), savings: cents(draft.savings), limits: Object.fromEntries(categories.map(c => [c.id, cents(draft.limits[c.id])])), bills: Object.fromEntries(bills.map(b => [b.id, cents(draft.bills[b.id])])) }
    if (await commit(next, 'Monthly plan saved.')) settings.value?.close()
  } catch (e) { error.value = String(e instanceof Error ? e.message : e) }
}
async function downloadBackup() {
  try {
    const payload = JSON.stringify(data.value, null, 2)
    if (native) { const path = await save({ defaultPath: 'Budetau-backup-' + today + '.json', filters: [{ name: 'Budetau backup', extensions: ['json'] }] }); if (!path) return; await invoke('write_basic_backup', { path, payload }) }
    else { const url = URL.createObjectURL(new Blob([payload], { type: 'application/json' })); const a = document.createElement('a'); a.href = url; a.download = 'Budetau-backup-' + today + '.json'; a.click(); setTimeout(() => URL.revokeObjectURL(url), 1000) }
    showMessage('Backup saved.')
  } catch (e) { error.value = 'Could not save the backup. ' + String(e) }
}
function stageRestore(payload: string) {
  pendingRestore.value = validateData(JSON.parse(payload)); restoreDialog.value?.showModal()
}
async function chooseRestore() {
  if (!native) { fileInput.value?.click(); return }
  try { const path = await open({ multiple: false, filters: [{ name: 'Budetau backup', extensions: ['json'] }] }); if (typeof path === 'string') stageRestore(await invoke<string>('read_basic_backup', { path })) }
  catch (e) { error.value = 'Could not read that backup. ' + String(e) }
}
async function readFile(event: Event) {
  const input = event.target as HTMLInputElement
  try { const file = input.files?.[0]; if (file) { if (file.size > 20000000) throw new Error('Backup is too large.'); stageRestore(await file.text()) } }
  catch (e) { error.value = 'Could not read that backup. ' + String(e) }
  finally { input.value = '' }
}
async function restoreBackup() {
  if (pendingRestore.value && await commit(pendingRestore.value, 'Backup restored.')) { pendingRestore.value = null; lastDeleted.value = null; restoreDialog.value?.close(); cancelEdit() }
}
function categoryName(entry: Entry) { return entry.kind === 'payment' ? 'Card payment · excluded' : [...categories, ...bills].find(c => c.id === entry.category)?.name || 'Other' }
function percent(spent: number, limit: number) { return Math.min(100, Math.max(0, limit ? spent / limit * 100 : spent > 0 ? 100 : 0)) }
function nearLimit(spent: number, limit: number) { return limit > 0 && spent >= limit * 0.85 && spent <= limit }
function dateLabel(date: string) { return new Date(date + 'T12:00:00').toLocaleDateString('en-US', { weekday: 'short', month: 'short', day: 'numeric' }) }
function merchant(entry: Entry) {
  const names: [RegExp, string][] = [[/^COSTCO GAS/i, 'Costco Gas'], [/^COSTCO WHSE/i, 'Costco'], [/^PROGRESSIVE INS/i, 'Progressive'], [/^WHOLEFDS/i, 'Whole Foods'], [/^PUBLIX (?:#|SUPERMARKETS)/i, 'Publix'], [/^ALDI \d/i, 'ALDI'], [/^Amazon|^AMAZON/i, 'Amazon'], [/^SUNPASS\*/i, 'SunPass'], [/^Payment Thank You/i, 'Chase card payment']]
  return names.find(([pattern]) => pattern.test(entry.note))?.[1] ?? (entry.note || categoryName(entry))
}
const storageLabel = native ? 'Saved on this Mac' : 'Saved in this browser'
</script>

<template>
  <main class="budget-app">
    <header class="app-header">
      <div class="brand"><h1>Budetau<span aria-hidden="true">.</span></h1><p>Your monthly budget</p></div>
      <div class="header-actions">
        <div class="month-navigation">
          <button class="month-step" type="button" aria-label="Previous month" :disabled="busy || month === '1900-01'" @click="shiftMonth(-1)"><svg aria-hidden="true" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="m14 6-6 6 6 6" /></svg></button>
          <label class="month-control"><span class="sr-only">Budget month</span><select v-model="month" aria-label="Budget month" :disabled="busy" @change="changeMonth"><option v-for="option in monthOptions" :key="option.value" :value="option.value">{{ option.label }}</option></select></label>
          <button class="month-step" type="button" aria-label="Next month" :disabled="busy || month === '2200-12'" @click="shiftMonth(1)"><svg aria-hidden="true" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="m10 6 6 6-6 6" /></svg></button>
        </div>
        <button class="button secondary" :disabled="!ready || busy" @click="openSettings">Edit budget</button>
        <button class="button primary" :disabled="!ready || busy" @click="openEntry"><span aria-hidden="true">＋</span> Add expense</button>
      </div>
    </header>

    <p v-if="error" class="notice error" role="alert">{{ error }} <button class="text-button" @click="error = ''">Dismiss</button></p>
    <p v-if="!ready && !error" class="notice">Opening your budget…</p>
    <section class="panel overview" aria-label="Monthly overview">
      <div class="main-number"><p class="eyebrow">Available to spend this month</p><div :class="['hero-amount', { negative: summary.remaining < 0 }]">{{ money(summary.remaining) }}</div><p class="supporting">Based on planned income and recorded expenses.</p></div>
      <dl class="overview-breakdown"><div><dt>Planned take-home pay</dt><dd>{{ money(summary.income) }}</dd></div><div><dt>Set aside for bills</dt><dd>{{ money(-summary.billReserve) }}</dd></div><div><dt>Set aside for savings</dt><dd>{{ summary.plan.savings ? money(-summary.plan.savings) : money(0) }}</dd></div><div class="spending-line"><dt>Everyday spending</dt><dd>{{ summary.spent ? money(-summary.spent) : money(0) }}</dd></div></dl>
    </section>
    <p v-if="data.coverage?.[month]" class="coverage-note"><svg aria-hidden="true" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"><circle cx="12" cy="12" r="9"/><path d="M12 11v6m0-10v1"/></svg>{{ data.coverage[month].note }}</p>

    <div class="workspace">
      <section class="panel categories-panel">
        <div class="section-heading"><h2>Category budgets</h2><span>Remaining</span></div>
        <div v-for="row in summary.rows" :key="row.id" :class="['category-row', { 'is-over': row.remaining < 0, 'is-near': nearLimit(row.spent, row.limit) }]">
          <div class="category-top"><strong>{{ row.name }}</strong><strong class="category-remaining">{{ money(Math.abs(row.remaining)) }}<span v-if="row.remaining < 0"> over</span></strong></div>
          <div class="meter" role="progressbar" :aria-label="row.name + ' spending'" :aria-valuemin="0" :aria-valuemax="100" :aria-valuenow="percent(row.spent, row.limit)"><div :style="{ width: percent(row.spent, row.limit) + '%' }"></div></div>
          <div class="category-meta"><span>{{ money(row.spent) }} spent of {{ money(row.limit) }}</span><span v-if="row.remaining < 0" class="budget-warning">Over budget</span><span v-else-if="nearLimit(row.spent, row.limit)" class="budget-warning">Near limit</span></div>
        </div>
        <div class="plan-cushion"><div class="row"><strong>Unallocated budget</strong><strong :class="{ negative: summary.cushion < 0 }">{{ money(summary.cushion) }}</strong></div><p>{{ summary.cushion < 0 ? 'Your category budgets exceed the amount available in your plan.' : 'Included in your available amount; not assigned to a category.' }}</p></div>
      </section>

      <aside class="side-panels">
        <section class="panel bills-panel" aria-label="Bills and savings">
          <h2>Bills & savings</h2><p class="card-amount">{{ money(summary.billReserve) }}</p><p class="supporting">Reserved for monthly bills</p>
          <p class="paycheck-tip">Set aside <strong>{{ money(summary.billsPerPaycheck) }}</strong> from each of your first two paychecks.</p>
          <details class="bill-details"><summary>Bill breakdown</summary><dl class="bill-list"><div v-for="bill in summary.billRows" :key="bill.id"><dt>{{ bill.name }}</dt><dd>{{ money(bill.amount) }} planned<span>{{ money(bill.paid) }} recorded</span></dd></div></dl><p class="small-note">Bills count once. Recorded amounts above the plan increase the reserve.</p></details>
          <div class="savings-row"><span>Savings reserved</span><strong>{{ money(summary.plan.savings) }}</strong></div>
        </section>
        <section class="panel one-off-panel" aria-label="One-off purchases">
          <div class="section-heading"><h2>One-off purchases</h2><span class="count-tag">{{ summary.extraEntries.length }}</span></div><p class="card-amount">{{ money(summary.extrasTotal) }}</p><p class="supporting">Already included in your spending.</p>
          <p v-if="summary.extraRefunds" class="small-note">{{ money(summary.extraPurchases) }} purchased · {{ money(summary.extraRefunds) }} refunded</p>
          <p v-if="!summary.extraEntries.length" class="empty-one-off">Mark part or all of a receipt as a one-off in the expense form.</p>
          <ul v-else class="one-off-list"><li v-for="entry in summary.extraEntries" :key="entry.id"><button class="one-off-item" :disabled="busy" :aria-label="'View one-off: ' + entry.extra.note" @click="showDetails(entry)"><span><strong>{{ entry.extra.note }}</strong><small>{{ dateLabel(entry.date) }} · {{ merchant(entry) }}<template v-if="entry.kind === 'refund'"> · Refund</template></small></span><strong :class="{ refund: entry.kind === 'refund' }">{{ entry.kind === 'refund' ? '−' : '' }}{{ money(entry.extra.amount) }}</strong><span class="chevron" aria-hidden="true">›</span></button></li></ul>
        </section>
      </aside>
    </div>

    <section class="panel history-panel" aria-label="Transactions this month">
      <div class="section-heading"><h2>Transactions</h2><span>{{ entries.length }} this month</span></div>
      <p v-if="!entries.length" class="empty-history">No transactions for {{ monthLabel }}. Add an expense to start tracking.</p>
      <section v-for="group in entryGroups" :key="group.date" class="transaction-group" :aria-label="dateLabel(group.date)">
        <h3>{{ dateLabel(group.date) }}</h3>
        <ul class="transaction-list"><li v-for="entry in group.entries" :key="entry.id" class="transaction-row">
          <div class="transaction-description"><div class="merchant-line"><strong>{{ merchant(entry) }}</strong><span v-if="entry.extra" class="one-off-tag">One-off</span><span v-if="entry.kind === 'refund'" class="refund-tag">Refund</span></div><p v-if="entry.extra" class="extra-marker">{{ entry.extra.note }} · {{ money(entry.extra.amount) }} {{ entry.kind === 'refund' ? 'of refund' : 'of receipt' }}</p><p class="mobile-category">{{ categoryName(entry) }}</p></div>
          <span class="transaction-category">{{ categoryName(entry) }}</span><strong :class="['transaction-amount', { refund: entry.kind === 'refund', 'excluded-amount': entry.kind === 'payment' }]">{{ entry.kind === 'refund' ? '−' : '' }}{{ money(entry.amount) }}</strong>
          <button class="details-button" :disabled="busy" :aria-label="'View ' + merchant(entry) + ' details, ' + dateLabel(entry.date) + ', ' + money(entry.amount)" @click="showDetails(entry)"><span aria-hidden="true">⋯</span></button>
        </li></ul>
      </section>
    </section>

    <footer><span class="storage-label"><span class="storage-dot" aria-hidden="true"></span>{{ storageLabel }}<template v-if="!native"> · Browser and desktop records are separate.</template></span><div><button class="text-button" :disabled="!ready || busy" @click="downloadBackup">Download backup</button><button class="text-button" :disabled="!ready || busy" @click="chooseRestore">Restore backup</button></div></footer>
    <input ref="fileInput" class="sr-only" type="file" accept=".json,application/json" aria-label="Choose budget backup" @change="readFile" />
    <div v-if="message || lastDeleted" class="toast" role="status" aria-live="polite"><span>{{ message || 'Transaction deleted.' }}</span><button v-if="lastDeleted" class="text-button" :disabled="busy" @click="undoDelete">Undo delete</button><button v-else class="toast-dismiss" aria-label="Dismiss confirmation" @click="message = ''">×</button></div>

    <dialog ref="entryDialog" class="entry-drawer" aria-labelledby="entry-title" @close="cancelEdit">
      <form @submit.prevent="addEntry">
        <div class="dialog-heading"><div><p class="dialog-eyebrow">{{ monthLabel }}</p><h2 id="entry-title">{{ form.id ? 'Edit transaction' : 'Add expense' }}</h2></div><button type="button" class="close-button" aria-label="Close expense form" :disabled="busy" @click="entryDialog?.close()">×</button></div>
        <fieldset :disabled="!ready || busy">
          <label class="amount-label">Full amount ($)<input ref="amountInput" v-model="form.amount" name="amount" inputmode="decimal" placeholder="0.00" autocomplete="off" required autofocus /></label>
          <label>Merchant or description <span class="muted">(optional)</span><input v-model="form.note" name="note" placeholder="e.g. Publix" maxlength="200" /></label>
          <div class="form-row"><label v-if="form.kind !== 'payment'">Category<select v-model="form.category"><optgroup label="Everyday spending"><option v-for="c in categories" :key="c.id" :value="c.id">{{ c.name }}</option></optgroup><optgroup label="Recurring bills"><option v-for="b in bills" :key="b.id" :value="b.id">{{ b.name }}</option></optgroup></select></label><label>Date<input v-model="form.date" type="date" min="1900-01-01" max="2200-12-31" required /></label></div>
          <details ref="entryOptions" class="entry-options"><summary>More options <span>{{ form.kind === 'payment' ? 'Card payment' : form.kind === 'refund' ? 'Refund' : form.hasExtra ? 'One-off purchase' : 'One-offs, refunds & card payments' }}</span></summary>
            <label>Entry type<select v-model="form.kind" @change="changeKind"><option value="purchase">Purchase / bill</option><option value="refund">Refund</option><option value="payment">Card payment</option></select></label>
            <p v-if="form.kind === 'payment'" class="payment-note">Card payments stay in your history and are excluded from spending.</p>
            <div v-else class="extra-fields"><label class="extra-toggle"><input v-model="form.hasExtra" type="checkbox" @change="changeExtra" /><span>{{ form.kind === 'refund' ? 'Refund for a one-off purchase' : 'Includes a one-off purchase' }}</span></label><div v-if="form.hasExtra" class="extra-details"><label>One-off amount ($)<input v-model="form.extraAmount" name="extraAmount" inputmode="decimal" placeholder="0.00" required /></label><label>One-off item<input v-model="form.extraNote" name="extraNote" placeholder="e.g. Air purifier" maxlength="200" required /></label><p>Part of the full receipt amount above. Counts only once.</p></div></div>
          </details>
          <p v-if="error" class="notice error" role="alert">{{ error }}</p>
          <div class="form-actions"><button class="button primary" type="submit">{{ busy ? 'Saving…' : form.id ? 'Save changes' : 'Save expense' }}</button><button class="button secondary" type="button" @click="entryDialog?.close()">Cancel</button></div>
        </fieldset>
      </form>
    </dialog>

    <dialog ref="detailsDialog" class="transaction-dialog" aria-labelledby="transaction-title">
      <template v-if="selectedEntry"><div class="dialog-heading"><h2 id="transaction-title">Transaction details</h2><button type="button" class="close-button" aria-label="Close transaction details" :disabled="busy" @click="detailsDialog?.close()">×</button></div><h3 class="detail-merchant">{{ merchant(selectedEntry) }}</h3><p :class="['detail-amount', { refund: selectedEntry.kind === 'refund' }]">{{ selectedEntry.kind === 'refund' ? '−' : '' }}{{ money(selectedEntry.amount) }}</p>
        <dl class="transaction-facts"><div><dt>Date</dt><dd>{{ dateLabel(selectedEntry.date) }}, {{ selectedEntry.date.slice(0, 4) }}</dd></div><div><dt>Category</dt><dd>{{ categoryName(selectedEntry) }}</dd></div><div><dt>Entry type</dt><dd>{{ selectedEntry.kind === 'payment' ? 'Card payment · excluded' : selectedEntry.kind === 'refund' ? 'Refund' : 'Purchase / bill' }}</dd></div><div v-if="selectedEntry.note"><dt>Original description</dt><dd>{{ selectedEntry.note }}</dd></div></dl>
        <div v-if="selectedEntry.extra" class="detail-extra"><span class="one-off-tag">One-off{{ selectedEntry.kind === 'refund' ? ' refund' : '' }}</span><div class="row"><strong>{{ selectedEntry.extra.note }}</strong><strong>{{ money(selectedEntry.extra.amount) }}</strong></div><p>Included in the {{ money(selectedEntry.amount) }} {{ selectedEntry.kind === 'refund' ? 'refund' : 'receipt' }} total.</p></div>
        <p v-if="selectedEntry.kind === 'payment'" class="small-note">This payment does not reduce your available budget.</p><p v-else-if="bills.some(b => b.id === selectedEntry?.category)" class="small-note">Included in your bill reserve. It counts once.</p>
        <p v-if="error" class="notice error" role="alert">{{ error }}</p>
        <div class="detail-actions"><button class="button primary" :disabled="busy" @click="editEntry(selectedEntry)">Edit transaction</button><button class="text-button delete" :disabled="busy" @click="removeEntry(selectedEntry.id)">{{ pendingDelete === selectedEntry.id ? 'Confirm delete' : 'Delete transaction' }}</button></div><p v-if="pendingDelete === selectedEntry.id" class="small-note">Delete this transaction? You can undo the deletion afterward. <button class="text-button" @click="pendingDelete = ''">Keep transaction</button></p>
      </template>
    </dialog>

    <dialog ref="settings" class="settings-dialog" aria-labelledby="settings-title"><form @submit.prevent="saveSettings"><div class="dialog-heading"><div><p class="dialog-eyebrow">{{ monthLabel }}</p><h2 id="settings-title">Edit monthly budget</h2></div><button class="close-button" type="button" aria-label="Close budget settings" :disabled="busy" @click="settings?.close()">×</button></div><p class="small-note">Changes apply to this month. New months inherit the most recent earlier plan.</p><fieldset :disabled="busy"><h3>Income & savings</h3><div class="settings-grid"><label>Take-home per paycheck<input v-model="draft.pay" inputmode="decimal" required /></label><label>Paychecks this month<select v-model="draft.paychecks"><option :value="2">2 paychecks</option><option :value="3">3 paychecks</option></select></label><label>Other income<input v-model="draft.extra" inputmode="decimal" required /></label><label>Savings goal<input v-model="draft.savings" inputmode="decimal" required /></label></div><h3>Monthly bills</h3><div class="settings-grid"><label v-for="b in bills" :key="b.id">{{ b.name }}<input v-model="draft.bills[b.id]" inputmode="decimal" required /></label></div><h3>Category budgets</h3><div class="settings-grid"><label v-for="c in categories" :key="c.id">{{ c.name }}<input v-model="draft.limits[c.id]" inputmode="decimal" required /></label></div><p v-if="error" class="notice error" role="alert">{{ error }}</p><button class="button primary" type="submit">{{ busy ? 'Saving…' : 'Save monthly budget' }}</button></fieldset></form></dialog>
    <dialog ref="restoreDialog" class="restore-dialog" aria-labelledby="restore-title"><h2 id="restore-title">Replace your budget?</h2><p>This backup contains {{ pendingRestore?.entries.length }} entries. Restoring it replaces all current entries and monthly plans.</p><p v-if="error" class="notice error">{{ error }}</p><div class="dialog-actions"><button class="button secondary" :disabled="busy" @click="restoreDialog?.close()">Cancel</button><button class="button primary" :disabled="busy" @click="restoreBackup">Restore backup</button></div></dialog>
  </main>
</template>
