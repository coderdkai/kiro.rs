import type { BalanceResponse } from '@/types/api'

const API_KEY_STORAGE_KEY = 'adminApiKey'
const DARK_MODE_KEY = 'darkMode'
const PAGE_SIZE_KEY = 'pageSize'
const BALANCE_CACHE_PREFIX = 'balance_'
const BALANCE_CACHE_TTL = 5 * 60 * 1000 // 5 分钟

interface CachedBalance {
  data: BalanceResponse
  timestamp: number
}

export const storage = {
  // API Key
  getApiKey: () => localStorage.getItem(API_KEY_STORAGE_KEY),
  setApiKey: (key: string) => localStorage.setItem(API_KEY_STORAGE_KEY, key),
  removeApiKey: () => localStorage.removeItem(API_KEY_STORAGE_KEY),

  // 暗色模式
  getDarkMode: (): boolean => {
    const stored = localStorage.getItem(DARK_MODE_KEY)
    if (stored === null) {
      // 首次访问，检测系统偏好
      return window.matchMedia('(prefers-color-scheme: dark)').matches
    }
    return stored === 'true'
  },
  setDarkMode: (enabled: boolean) => {
    localStorage.setItem(DARK_MODE_KEY, String(enabled))
  },

  // 分页大小
  getPageSize: (): number => {
    const stored = localStorage.getItem(PAGE_SIZE_KEY)
    return stored ? parseInt(stored, 10) : 12
  },
  setPageSize: (size: number) => {
    localStorage.setItem(PAGE_SIZE_KEY, String(size))
  },

  // 余额缓存（带 TTL）
  getBalanceCache: (id: number): BalanceResponse | null => {
    try {
      const cached = localStorage.getItem(`${BALANCE_CACHE_PREFIX}${id}`)
      if (!cached) return null

      const parsed: CachedBalance = JSON.parse(cached)
      const now = Date.now()

      // 检查是否过期
      if (now - parsed.timestamp > BALANCE_CACHE_TTL) {
        localStorage.removeItem(`${BALANCE_CACHE_PREFIX}${id}`)
        return null
      }

      return parsed.data
    } catch {
      return null
    }
  },

  setBalanceCache: (id: number, data: BalanceResponse) => {
    try {
      const cached: CachedBalance = {
        data,
        timestamp: Date.now(),
      }
      localStorage.setItem(`${BALANCE_CACHE_PREFIX}${id}`, JSON.stringify(cached))
    } catch {
      // 忽略存储错误（可能是配额满了）
    }
  },

  clearBalanceCache: (id: number) => {
    localStorage.removeItem(`${BALANCE_CACHE_PREFIX}${id}`)
  },

  clearAllBalanceCache: () => {
    const keys = Object.keys(localStorage)
    keys.forEach(key => {
      if (key.startsWith(BALANCE_CACHE_PREFIX)) {
        localStorage.removeItem(key)
      }
    })
  },
}
