import { describe, expect, it } from 'vitest'
import { clone, defaultPlan, emptyData, summarize, validateData, validateEntry } from './basicBudget'
import type { Entry } from './basicBudget'

const costco: Entry = { id: 'costco', date: '2026-10-04', amount: 12148, category: 'shopping', kind: 'purchase', note: 'Costco' }

describe('monthly extras', () => {
  it('tracks part of a receipt without changing its category spending or remaining budget', () => {
    const data = emptyData()
    data.entries = [clone(costco)]
    const before = summarize(data, '2026-10')
    data.entries[0].extra = { amount: 10000, note: 'Air purifier' }
    const after = summarize(validateData(data), '2026-10')
    expect(after.extrasTotal).toBe(10000)
    expect(after.extraEntries).toHaveLength(1)
    expect(after.spent).toBe(12148)
    expect(after.rows.find(row => row.id === 'shopping')?.spent).toBe(12148)
    expect(after.remaining).toBe(before.remaining)
    expect(after.cushion).toBe(before.cushion)
  })

  it('nets marked refunds in their own month, excludes payments, and preserves bill reserves', () => {
    const data = emptyData()
    data.months['2026-10'] = defaultPlan()
    data.entries = [
      { ...costco, extra: { amount: 10000, note: 'Air purifier' } },
      { ...costco, id: 'refund', date: '2026-10-08', kind: 'refund', amount: 2500, extra: { amount: 2000, note: 'Partial return' } },
      { ...costco, id: 'payment', kind: 'payment', category: '', amount: 40000 },
      { ...costco, id: 'insurance', category: 'insurance', amount: 18833 },
      { ...costco, id: 'next-month', date: '2026-11-01', extra: { amount: 5000, note: 'November extra' } },
    ]
    const summary = summarize(validateData(data), '2026-10')
    expect(summary.extrasTotal).toBe(8000)
    expect(summary.extraPurchases).toBe(10000)
    expect(summary.extraRefunds).toBe(2000)
    expect(summary.extraEntries.map(entry => entry.id)).toEqual(['refund', 'costco'])
    expect(summary.spent).toBe(9648)
    expect(summary.billReserve).toBe(235833)
    expect(summary.remaining).toBe(440000 - 235833 - 9648)
    expect(summarize(data, '2026-11').extrasTotal).toBe(5000)
  })

  it('restores older backups and round-trips marked extras without changing other data', () => {
    const oldBackup = { ...emptyData(), entries: [clone(costco)] }
    expect(summarize(validateData(oldBackup), '2026-10').extrasTotal).toBe(0)
    oldBackup.entries[0].extra = { amount: 10000, note: 'Air purifier' }
    const restored = validateData(JSON.parse(JSON.stringify(oldBackup)))
    expect(restored).toEqual(oldBackup)
    restored.entries[0].extra = undefined
    const unmarked = summarize(validateData(restored), '2026-10')
    expect(unmarked.extrasTotal).toBe(0)
    expect(unmarked.spent).toBe(12148)
  })

  it('rejects invalid splits and marking a card payment as an extra', () => {
    for (const amount of [0, -1, 12149, 1.5, NaN]) {
      expect(() => validateEntry({ ...costco, extra: { amount, note: 'Air purifier' } })).toThrow('extra amount')
    }
    expect(() => validateEntry({ ...costco, extra: { amount: 10000, note: ' ' } })).toThrow('Describe')
    expect(() => validateEntry({ ...costco, kind: 'payment', category: '', extra: { amount: 10000, note: 'Air purifier' } })).toThrow('Card payments')
  })
})
