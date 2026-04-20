import { useState, useEffect } from 'react'
import { KeyRound } from 'lucide-react'
import { storage } from '@/lib/storage'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'

interface LoginPageProps {
  onLogin: () => void
}

export function LoginPage({ onLogin }: LoginPageProps) {
  const [apiKey, setApiKey] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  useEffect(() => {
    const savedKey = storage.getApiKey()
    if (savedKey) {
      setApiKey(savedKey)
    }
  }, [])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!apiKey.trim()) return

    setLoading(true)
    setError('')

    try {
      // 使用 fetch 而非 axios 实例，避免触发 401 拦截器
      const res = await fetch('/api/admin/credentials', {
        headers: { 'x-api-key': apiKey.trim() },
      })
      if (!res.ok) {
        if (res.status === 401) {
          setError('API Key 无效，请检查后重试')
        } else {
          setError('服务器错误，请稍后重试')
        }
        return
      }
      // 验证成功，保存并登录
      storage.setApiKey(apiKey.trim())
      onLogin()
    } catch {
      setError('无法连接服务器，请检查网络')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-background p-4">
      <Card className="w-full max-w-md">
        <CardHeader className="text-center">
          <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-primary/10">
            <KeyRound className="h-6 w-6 text-primary" />
          </div>
          <CardTitle className="text-2xl">Kiro Admin</CardTitle>
          <CardDescription>
            请输入 Admin API Key 以访问管理面板
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-4">
            <div className="space-y-2">
              <Input
                type="password"
                placeholder="Admin API Key"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                className="text-center"
              />
            </div>
            <Button type="submit" className="w-full" disabled={!apiKey.trim() || loading}>
              {loading ? '验证中...' : '登录'}
            </Button>
            {error && (
              <p className="text-sm text-red-500 text-center">{error}</p>
            )}
          </form>
        </CardContent>
      </Card>
    </div>
  )
}
