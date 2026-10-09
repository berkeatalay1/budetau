export const categories = [
  { id: 'groceries', name: 'Groceries', limit: 75000, color: '#2563eb' },
  { id: 'dining', name: 'Dining & entertainment', limit: 20000, color: '#7654cb' },
  { id: 'shopping', name: 'Shopping & household', limit: 25000, color: '#d17b16' },
  { id: 'transport', name: 'Gas & tolls', limit: 22500, color: '#16816b' },
  { id: 'other', name: 'Other', limit: 13700, color: '#677b92' },
] as const
export const bills = [
  { id: 'rent', name: 'Rent', amount: 184500 },
  { id: 'car', name: 'Car payment', amount: 20000 },
  { id: 'internet', name: 'Internet', amount: 5000 },
  { id: 'electricity', name: 'Electricity', amount: 7500 },
  { id: 'insurance', name: 'Car insurance', amount: 18833 },
] as const
export type Entry = { id: string; date: string; amount: number; category: string; kind: 'purchase' | 'refund' | 'payment'; note: string; extra?: { amount: number; note: string } }
export type Plan = { pay: number; paychecks: number; extra: number; savings: number; limits: Record<string, number>; bills: Record<string, number> }
export type BudgetData = { version: 1; months: Record<string, Plan>; entries: Entry[]; coverage?: Record<string, { start: string; through: string; note: string }> }
export function clone<T>(value: T): T { return JSON.parse(JSON.stringify(value)) as T }
export function defaultPlan(): Plan {
  return { pay: 220000, paychecks: 2, extra: 0, savings: 0, limits: Object.fromEntries(categories.map(c => [c.id, c.limit])), bills: Object.fromEntries(bills.map(b => [b.id, b.amount])) }
}
export function emptyData(): BudgetData { return { version: 1, months: {}, entries: [] } }
export function planFor(data: BudgetData, month: string): Plan {
  const earlier = Object.keys(data.months).filter(m => m <= month).sort().pop()
  return clone(data.months[month] ?? (earlier ? data.months[earlier] : defaultPlan()))
}
export function cents(value: string): number {
  if (!/^\d+(\.\d{1,2})?$/.test(value.trim())) throw new Error('Enter an amount with no more than two decimal places.')
  const [whole, fraction = ''] = value.trim().split('.')
  const amount = Number(whole) * 100 + Number(fraction.padEnd(2, '0'))
  if (!Number.isSafeInteger(amount) || amount > 10000000000) throw new Error('That amount is too large.')
  return amount
}
export function isDate(value: string): boolean {
  const date = new Date(value + 'T12:00:00Z')
  return /^\d{4}-\d{2}-\d{2}$/.test(value) && Number(value.slice(0, 4)) >= 1900 && Number(value.slice(0, 4)) <= 2200 && !Number.isNaN(date.getTime()) && date.toISOString().slice(0, 10) === value
}
export function validateEntry(entry: Entry): void {
  if (!entry || typeof entry.id !== 'string' || !entry.id || typeof entry.date !== 'string' || !isDate(entry.date) || !Number.isSafeInteger(entry.amount) || entry.amount <= 0 || entry.amount > 10000000000 || !['purchase', 'refund', 'payment'].includes(entry.kind) || typeof entry.note !== 'string' || entry.note.length > 200) throw new Error('Check the entry’s date and amount.')
  if (entry.kind !== 'payment' && ![...categories, ...bills].some(c => c.id === entry.category)) throw new Error('Choose a valid category.')
  if (entry.kind === 'payment' && entry.category !== '') throw new Error('Card payments do not have a spending category.')
  if (entry.extra !== undefined) {
    if (entry.kind === 'payment') throw new Error('Card payments cannot be marked as extra purchases.')
    if (!entry.extra || typeof entry.extra !== 'object' || Array.isArray(entry.extra) || !Number.isSafeInteger(entry.extra.amount) || entry.extra.amount <= 0 || entry.extra.amount > entry.amount) throw new Error('The extra amount must be greater than zero and no more than the full entry amount.')
    if (typeof entry.extra.note !== 'string' || !entry.extra.note.trim() || entry.extra.note.length > 200) throw new Error('Describe the extra purchase in 200 characters or less.')
  }
}
export function validateData(value: unknown): BudgetData {
  const data = value as BudgetData
  if (!data || data.version !== 1 || !data.months || typeof data.months !== 'object' || Array.isArray(data.months) || !Array.isArray(data.entries) || data.entries.length > 50000) throw new Error('This is not a Budetau basic backup.')
  const validMoney = (v: unknown) => typeof v === 'number' && Number.isSafeInteger(v) && v >= 0 && v <= 10000000000
  for (const [month, p] of Object.entries(data.months)) {
    if (!/^\d{4}-(0[1-9]|1[0-2])$/.test(month) || !p || !validMoney(p.pay) || !validMoney(p.extra) || !validMoney(p.savings) || ![2, 3].includes(p.paychecks) || !p.limits || !p.bills || !categories.every(c => validMoney(p.limits[c.id])) || !bills.every(b => validMoney(p.bills[b.id]))) throw new Error('The backup contains an invalid monthly plan.')
  }
  if (data.coverage !== undefined) {
    if (!data.coverage || typeof data.coverage !== 'object' || Array.isArray(data.coverage)) throw new Error('Invalid statement coverage.')
    for (const [month, coverage] of Object.entries(data.coverage)) {
      if (!/^\d{4}-(0[1-9]|1[0-2])$/.test(month) || !coverage || typeof coverage.start !== 'string' || typeof coverage.through !== 'string' || !isDate(coverage.start) || !isDate(coverage.through) || coverage.start > coverage.through || typeof coverage.note !== 'string' || coverage.note.length > 300) throw new Error('Invalid statement coverage.')
    }
  }
  const ids = new Set<string>()
  for (const entry of data.entries) { validateEntry(entry); if (ids.has(entry.id)) throw new Error('The backup contains duplicate entries.'); ids.add(entry.id) }
  return clone(data)
}
export function summarize(data: BudgetData, month: string) {
  const plan = planFor(data, month)
  const entries = data.entries.filter(e => e.date.slice(0, 7) === month)
  const net = (id: string) => entries.reduce((sum, e) => sum + (e.category === id && e.kind !== 'payment' ? e.amount * (e.kind === 'refund' ? -1 : 1) : 0), 0)
  const rows = categories.map(c => ({ ...c, limit: plan.limits[c.id], spent: net(c.id), remaining: plan.limits[c.id] - net(c.id) }))
  const billRows = bills.map(b => ({ ...b, amount: plan.bills[b.id], paid: net(b.id), reserved: Math.max(plan.bills[b.id], net(b.id)) }))
  const billTotal = billRows.reduce((sum, b) => sum + b.amount, 0)
  const billReserve = billRows.reduce((sum, b) => sum + b.reserved, 0)
  const income = plan.pay * plan.paychecks + plan.extra
  const spent = rows.reduce((sum, c) => sum + c.spent, 0)
  const limits = rows.reduce((sum, c) => sum + c.limit, 0)
  const extraEntries = entries.filter((e): e is Entry & { extra: NonNullable<Entry['extra']> } => e.kind !== 'payment' && e.extra !== undefined).sort((a, b) => b.date.localeCompare(a.date))
  const extraPurchases = extraEntries.filter(e => e.kind === 'purchase').reduce((sum, e) => sum + e.extra.amount, 0)
  const extraRefunds = extraEntries.filter(e => e.kind === 'refund').reduce((sum, e) => sum + e.extra.amount, 0)
  return { plan, entries, rows, billRows, billTotal, billReserve, income, spent, limits, extraEntries, extraPurchases, extraRefunds, extrasTotal: extraPurchases - extraRefunds, remaining: income - billReserve - plan.savings - spent, cushion: income - billReserve - plan.savings - limits, billsPerPaycheck: Math.ceil(billTotal / 2) }
}
