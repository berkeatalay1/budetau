/**
 * Formatting helpers. Money is stored as INTEGER minor units (cents);
 * always format for display, never show raw cents.
 */

/** Format cents as a currency string, e.g. 123456 -> "$1,234.56". */
export function formatCents(cents: number, currency = 'USD', locale = 'en-US'): string {
  return new Intl.NumberFormat(locale, { style: 'currency', currency }).format(cents / 100)
}

/**
 * Format cents with an explicit leading sign, e.g. +$4,200.00 / -$25.00.
 * Useful for transaction rows where direction matters.
 */
export function formatSignedCents(cents: number, currency = 'USD', locale = 'en-US'): string {
  const sign = cents > 0 ? '+' : cents < 0 ? '-' : ''
  return sign + formatCents(Math.abs(cents), currency, locale)
}

/** Format an ISO date (YYYY-MM-DD) as e.g. "Jun 25". */
export function formatDateShort(iso: string, locale = 'en-US'): string {
  const d = new Date(iso + 'T00:00:00')
  return new Intl.DateTimeFormat(locale, { month: 'short', day: 'numeric' }).format(d)
}

/** Format an ISO date as a day group header, e.g. "Today", "Yesterday", or "Wed, Jun 25". */
export function formatDayHeader(iso: string, locale = 'en-US'): string {
  const d = new Date(iso + 'T00:00:00')
  const today = new Date()
  today.setHours(0, 0, 0, 0)
  const diffDays = Math.round((today.getTime() - d.getTime()) / 86_400_000)
  if (diffDays === 0) return 'Today'
  if (diffDays === 1) return 'Yesterday'
  return new Intl.DateTimeFormat(locale, {
    weekday: 'short',
    month: 'short',
    day: 'numeric',
  }).format(d)
}

/** Today's date as an ISO YYYY-MM-DD string (local time). */
export function todayISO(): string {
  const d = new Date()
  const off = d.getTimezoneOffset()
  return new Date(d.getTime() - off * 60_000).toISOString().slice(0, 10)
}
