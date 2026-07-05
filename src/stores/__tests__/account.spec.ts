import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { Account } from '../../types'

// Mock the IPC layer so the store never touches Tauri `invoke`.
vi.mock('../../api', () => ({
  listAccounts: vi.fn(),
  createAccount: vi.fn(),
  updateAccount: vi.fn(),
  deleteAccount: vi.fn(),
  recomputeBalances: vi.fn(),
}))

import { listAccounts, createAccount, deleteAccount, recomputeBalances } from '../../api'
import { useAccountStore } from '../account'

function acct(id: number, name: string, balance: number): Account {
  return { id, profileId: 1, name, balance, currency: 'USD', type: 'checking' }
}

describe('account store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('fetchAll loads accounts and clears loading/error', async () => {
    vi.mocked(listAccounts).mockResolvedValue([acct(1, 'Checking', 1000)])
    const store = useAccountStore()

    await store.fetchAll()

    expect(store.data.map((a) => a.name)).toEqual(['Checking'])
    expect(store.isLoading).toBe(false)
    expect(store.error).toBeNull()
  })

  it('fetchAll captures the error message on failure', async () => {
    vi.mocked(listAccounts).mockRejectedValue(new Error('db locked'))
    const store = useAccountStore()

    await store.fetchAll()

    expect(store.error).toBe('db locked')
    expect(store.isLoading).toBe(false)
  })

  it('add creates then re-fetches the list', async () => {
    vi.mocked(createAccount).mockResolvedValue(acct(2, 'Savings', 0))
    vi.mocked(listAccounts).mockResolvedValue([acct(1, 'Checking', 1000), acct(2, 'Savings', 0)])
    const store = useAccountStore()

    const created = await store.add({
      profileId: 1,
      name: 'Savings',
      currency: 'USD',
      type: 'savings',
    })

    expect(created.name).toBe('Savings')
    expect(createAccount).toHaveBeenCalledOnce()
    expect(store.data).toHaveLength(2)
  })

  it('remove deletes then re-fetches', async () => {
    vi.mocked(deleteAccount).mockResolvedValue(undefined)
    vi.mocked(listAccounts).mockResolvedValue([])
    const store = useAccountStore()

    await store.remove(1)

    expect(deleteAccount).toHaveBeenCalledWith(1)
    expect(store.data).toEqual([])
  })

  it('recompute replaces data with the corrected balances it returns', async () => {
    const corrected = [acct(1, 'Checking', 97500), acct(2, 'Savings', 80000)]
    vi.mocked(recomputeBalances).mockResolvedValue(corrected)
    const store = useAccountStore()
    // Seed obviously-wrong balances first.
    store.data = [acct(1, 'Checking', 999999), acct(2, 'Savings', -50000)]

    const result = await store.recompute()

    expect(result).toEqual(corrected)
    expect(store.data.map((a) => a.balance)).toEqual([97500, 80000])
    // recompute uses the returned list directly, not a re-fetch.
    expect(listAccounts).not.toHaveBeenCalled()
  })
})
