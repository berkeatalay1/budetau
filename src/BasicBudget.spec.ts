import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import BasicBudget from './BasicBudget.vue'
import { defaultPlan, summarize } from './lib/basicBudget'
import type { BudgetData } from './lib/basicBudget'

vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => false, invoke: vi.fn() }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn(), save: vi.fn() }))

const fixture: BudgetData = {
  version: 1, months: { '2026-10': defaultPlan() }, entries: [
    { id: 'costco', date: '2026-10-04', amount: 12148, category: 'shopping', kind: 'purchase', note: 'COSTCO WHSE #0345 BOCA RATON FL', extra: { amount: 10000, note: 'Air purifier' } },
    { id: 'payment', date: '2026-10-06', amount: 50000, category: '', kind: 'payment', note: 'Payment Thank You-Mobile' },
  ],
}
let wrapper: ReturnType<typeof mount>
let stored: string
const write = vi.fn((key: string, value: string) => { if (key === 'budetau-basic-v1') stored = value })
const originalShow = Object.getOwnPropertyDescriptor(HTMLDialogElement.prototype, 'showModal')
const originalClose = Object.getOwnPropertyDescriptor(HTMLDialogElement.prototype, 'close')
beforeAll(() => {
  Object.defineProperty(HTMLDialogElement.prototype, 'showModal', { configurable: true, value: function(this: HTMLDialogElement) { this.open = true } })
  Object.defineProperty(HTMLDialogElement.prototype, 'close', { configurable: true, value: function(this: HTMLDialogElement) { this.open = false; this.dispatchEvent(new Event('close')) } })
})
afterAll(() => {
  if (originalShow) Object.defineProperty(HTMLDialogElement.prototype, 'showModal', originalShow)
  else Reflect.deleteProperty(HTMLDialogElement.prototype, 'showModal')
  if (originalClose) Object.defineProperty(HTMLDialogElement.prototype, 'close', originalClose)
  else Reflect.deleteProperty(HTMLDialogElement.prototype, 'close')
})
beforeEach(async () => {
  vi.useFakeTimers({ toFake: ['Date', 'setTimeout', 'clearTimeout'] })
  vi.setSystemTime(new Date('2026-10-09T12:00:00Z'))
  stored = JSON.stringify(fixture)
  write.mockClear()
  vi.stubGlobal('localStorage', { getItem: () => stored, setItem: write })
  wrapper = mount(BasicBudget)
  await flushPromises()
})
afterEach(() => { wrapper.unmount(); vi.useRealTimers(); vi.unstubAllGlobals() })

const button = (text: string) => wrapper.findAll('button').find(b => b.text() === text)!
const openCostco = async () => { await wrapper.get('[aria-label="View one-off: Air purifier"]').trigger('click'); await flushPromises() }

describe('budget dashboard and transaction flows', () => {
  it('keeps the form closed and shows clear totals, dates, tags, and original details', async () => {
    expect(wrapper.get('.entry-drawer').attributes('open')).toBeUndefined()
    expect(wrapper.get('.main-number').text()).toContain('Available to spend this month')
    expect(wrapper.get('.main-number').text()).toContain('$1,920.19')
    expect(wrapper.get('.plan-cushion').text()).toContain('Included in your available amount')
    expect(wrapper.get('.one-off-panel').text()).toContain('$100.00')
    expect(wrapper.findAll('.transaction-group')).toHaveLength(2)
    expect(wrapper.get('.history-panel').text()).toContain('Card payment · excluded')
    expect(wrapper.get('.history-panel').text()).not.toContain('Delete transaction')
    await openCostco()
    expect(wrapper.get('.transaction-dialog').attributes('open')).toBeDefined()
    expect(wrapper.get('.transaction-dialog').text()).toContain('COSTCO WHSE #0345 BOCA RATON FL')
    expect(wrapper.get('.transaction-dialog').text()).toContain('Included in the $121.48 receipt total')
  })

  it('opens existing one-offs for editing and preserves the receipt and budget totals', async () => {
    await openCostco(); await button('Edit transaction').trigger('click'); await flushPromises()
    const drawer = wrapper.get('.entry-drawer')
    expect(drawer.attributes('open')).toBeDefined()
    expect((drawer.get('[name="amount"]').element as HTMLInputElement).value).toBe('121.48')
    expect((drawer.get('details').element as HTMLDetailsElement).open).toBe(true)
    await drawer.get('[name="extraAmount"]').setValue('90.00')
    await drawer.get('form').trigger('submit'); await flushPromises()
    const saved = JSON.parse(stored) as BudgetData
    expect(saved.entries).toHaveLength(2)
    expect(saved.entries[0].extra?.amount).toBe(9000)
    expect(saved.entries[0].amount).toBe(12148)
    expect(summarize(saved, '2026-10').remaining).toBe(summarize(fixture, '2026-10').remaining)
    expect(drawer.attributes('open')).toBeUndefined()
    expect(wrapper.get('.toast').text()).toContain('Transaction updated')
    vi.advanceTimersByTime(4500)
    await wrapper.vm.$nextTick()
    expect(wrapper.find('.toast').exists()).toBe(false)
  })

  it('confirms deletion in details and restores the complete transaction with undo', async () => {
    await openCostco(); await button('Delete transaction').trigger('click')
    expect(JSON.parse(stored)).toEqual(fixture)
    await button('Confirm delete').trigger('click'); await flushPromises()
    expect(JSON.parse(stored).entries.map((e: { id: string }) => e.id)).toEqual(['payment'])
    expect(wrapper.get('.one-off-panel .card-amount').text()).toBe('$0.00')
    vi.advanceTimersByTime(10000); await wrapper.vm.$nextTick()
    expect(button('Undo delete').exists()).toBe(true)
    await button('Undo delete').trigger('click'); await flushPromises()
    expect(JSON.parse(stored)).toEqual(fixture)
    expect(wrapper.get('.one-off-panel .card-amount').text()).toBe('$100.00')
  })

  it('clears one-off fields when choosing a payment and when cancelling an edit', async () => {
    await openCostco(); await button('Edit transaction').trigger('click'); await flushPromises()
    const drawer = wrapper.get('.entry-drawer')
    await drawer.findAll('select')[1].setValue('payment')
    expect(drawer.text()).toContain('excluded from spending')
    expect(drawer.find('[name="extraAmount"]').exists()).toBe(false)
    await drawer.get('[aria-label="Close expense form"]').trigger('click')
    await button('＋ Add expense').trigger('click'); await flushPromises()
    expect((drawer.get('[name="amount"]').element as HTMLInputElement).value).toBe('')
    expect((drawer.get('details').element as HTMLDetailsElement).open).toBe(false)
    expect((drawer.get('input[type="checkbox"]').element as HTMLInputElement).checked).toBe(false)
    expect(write).not.toHaveBeenCalled()
  })

  it('keeps an edited transaction and its form intact when saving fails', async () => {
    await openCostco(); await button('Edit transaction').trigger('click'); await flushPromises()
    write.mockImplementationOnce(() => { throw new Error('Storage full') })
    const drawer = wrapper.get('.entry-drawer')
    await drawer.get('[name="amount"]').setValue('130.00')
    await drawer.get('form').trigger('submit'); await flushPromises()
    expect(drawer.attributes('open')).toBeDefined()
    expect(drawer.get('[role="alert"]').text()).toContain('Could not save')
    expect((drawer.get('[name="amount"]').element as HTMLInputElement).value).toBe('130.00')
    expect(JSON.parse(stored)).toEqual(fixture)
  })
})
