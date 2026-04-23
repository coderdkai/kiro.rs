import { useState, useRef, useCallback, useEffect } from 'react'
import { toast } from 'sonner'
import { useQueryClient } from '@tanstack/react-query'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { storage } from '@/lib/storage'

interface AutoRegisterDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

type RegisterStep = 'idle' | 'running' | 'success' | 'error'

interface RegisterResult {
  success: boolean
  email?: string
  credentialId?: number
  error?: string
}

export function AutoRegisterDialog({ open, onOpenChange }: AutoRegisterDialogProps) {
  const queryClient = useQueryClient()
  const [step, setStep] = useState<RegisterStep>('idle')
  const [logs, setLogs] = useState<string[]>([])
  const [result, setResult] = useState<RegisterResult | null>(null)
  const logEndRef = useRef<HTMLDivElement>(null)
  const abortRef = useRef<AbortController | null>(null)

  const appendLog = useCallback((line: string) => {
    setLogs(prev => [...prev, line])
  }, [])

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [logs])

  const startRegister = useCallback(async () => {
    setStep('running')
    setLogs([])
    setResult(null)

    const apiKey = storage.getApiKey()
    const controller = new AbortController()
    abortRef.current = controller

    try {
      const resp = await fetch('/api/admin/auto-register', {
        headers: {
          'x-api-key': apiKey || '',
          'Accept': 'text/event-stream',
        },
        signal: controller.signal,
      })

      if (!resp.ok) {
        const errText = await resp.text()
        let errMsg = `HTTP ${resp.status}`
        try {
          const errJson = JSON.parse(errText)
          errMsg = errJson.error?.message || errMsg
        } catch {
          errMsg = errText || errMsg
        }
        setStep('error')
        appendLog(`❌ ${errMsg}`)
        toast.error(errMsg)
        return
      }

      const reader = resp.body?.getReader()
      if (!reader) {
        setStep('error')
        appendLog('❌ 无法获取响应流')
        return
      }

      const decoder = new TextDecoder()
      let buffer = ''

      while (true) {
        const { done, value } = await reader.read()
        if (done) break

        buffer += decoder.decode(value, { stream: true })

        const lines = buffer.split('\n')
        buffer = lines.pop() || ''

        let currentEvent = ''
        let currentData = ''

        for (const line of lines) {
          if (line.startsWith('event: ')) {
            currentEvent = line.slice(7)
          } else if (line.startsWith('data: ')) {
            currentData = line.slice(6)
          } else if (line === '' && currentEvent) {
            if (currentEvent === 'log') {
              appendLog(currentData)
            } else if (currentEvent === 'error') {
              appendLog(`❌ ${currentData}`)
            } else if (currentEvent === 'result') {
              try {
                const data: RegisterResult = JSON.parse(currentData)
                setResult(data)
                if (data.success) {
                  setStep('success')
                  toast.success(`注册成功: ${data.email || ''}`)
                  queryClient.invalidateQueries({ queryKey: ['credentials'] })
                } else {
                  setStep('error')
                  toast.error(`注册失败: ${data.error || '未知错误'}`)
                }
              } catch {
                setStep('error')
              }
            }
            currentEvent = ''
            currentData = ''
          }
        }
      }
    } catch (e) {
      if ((e as Error).name !== 'AbortError') {
        setStep('error')
        appendLog(`❌ 连接失败: ${(e as Error).message}`)
        toast.error('注册连接失败')
      }
    } finally {
      abortRef.current = null
    }
  }, [appendLog, queryClient])

  const handleClose = useCallback(() => {
    if (abortRef.current) {
      abortRef.current.abort()
      abortRef.current = null
    }
    setStep('idle')
    setLogs([])
    setResult(null)
    onOpenChange(false)
  }, [onOpenChange])

  const handleOpenChange = useCallback((newOpen: boolean) => {
    if (!newOpen) {
      handleClose()
    } else {
      onOpenChange(newOpen)
    }
  }, [handleClose, onOpenChange])

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-[600px] max-h-[80vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>自动注册</DialogTitle>
        </DialogHeader>

        <div className="flex-1 min-h-0">
          {step === 'idle' && (
            <div className="text-center py-8">
              <p className="text-muted-foreground mb-4">
                自动创建 iCloud Hide My Email 邮箱并完成 AWS Builder ID 注册，注册完成后凭据自动添加到池子。
              </p>
              <p className="text-sm text-muted-foreground">
                过程约需 2-4 分钟，请确保 config.json 中已配置 register 相关参数。
              </p>
            </div>
          )}

          {(step === 'running' || step === 'success' || step === 'error') && (
            <div className="bg-muted/50 rounded-lg p-3 h-[400px] overflow-y-auto font-mono text-xs leading-relaxed">
              {logs.map((line, i) => (
                <div key={i} className="whitespace-pre-wrap break-all">
                  {line}
                </div>
              ))}
              <div ref={logEndRef} />
            </div>
          )}

          {result && (
            <div className={`mt-3 p-3 rounded-lg text-sm ${result.success ? 'bg-green-500/10 text-green-700 dark:text-green-400' : 'bg-red-500/10 text-red-700 dark:text-red-400'}`}>
              {result.success ? (
                <div>
                  <span className="font-semibold">注册成功</span>
                  {result.email && <span> - {result.email}</span>}
                  {result.credentialId && <span> (凭据 #{result.credentialId})</span>}
                </div>
              ) : (
                <div>
                  <span className="font-semibold">注册失败</span>
                  {result.error && <span> - {result.error}</span>}
                </div>
              )}
            </div>
          )}
        </div>

        <DialogFooter>
          {step === 'idle' && (
            <>
              <Button variant="outline" onClick={handleClose}>取消</Button>
              <Button onClick={startRegister}>开始注册</Button>
            </>
          )}
          {step === 'running' && (
            <Button variant="outline" onClick={handleClose}>取消</Button>
          )}
          {(step === 'success' || step === 'error') && (
            <>
              {step === 'error' && (
                <Button variant="outline" onClick={() => { setStep('idle'); setLogs([]); setResult(null) }}>
                  重试
                </Button>
              )}
              <Button onClick={handleClose}>关闭</Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
