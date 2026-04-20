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
import { Input } from '@/components/ui/input'
import { deviceFlowRegister, deviceFlowAuthorize, deviceFlowPoll, addCredential } from '@/api/credentials'
import { extractErrorMessage } from '@/lib/utils'
import type { DeviceFlowAuthorizeResponse } from '@/types/api'

interface DeviceLoginDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

type LoginType = 'social' | 'personal' | 'enterprise'
type FlowStep = 'idle' | 'registering' | 'authorizing' | 'polling' | 'success' | 'error'

export function DeviceLoginDialog({ open, onOpenChange }: DeviceLoginDialogProps) {
  const queryClient = useQueryClient()
  const [loginType, setLoginType] = useState<LoginType>('social')
  const [enterpriseStartUrl, setEnterpriseStartUrl] = useState('https://d-906600eb6f.awsapps.com/start')
  const [step, setStep] = useState<FlowStep>('idle')
  const [errorMessage, setErrorMessage] = useState('')
  const [authResponse, setAuthResponse] = useState<DeviceFlowAuthorizeResponse | null>(null)
  const [pollStatus, setPollStatus] = useState('')

  // 用 ref 存储轮询所需数据，避免闭包陷阱
  const pollTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const pollIntervalRef = useRef(2000)
  const pollingRef = useRef(false)
  const clientIdRef = useRef('')
  const clientSecretRef = useRef('')
  const deviceCodeRef = useRef('')
  const loginTypeRef = useRef<LoginType>('social')

  // 保持 ref 和 state 同步
  useEffect(() => { loginTypeRef.current = loginType }, [loginType])

  const resetState = useCallback(() => {
    if (pollTimerRef.current) {
      clearTimeout(pollTimerRef.current)
      pollTimerRef.current = null
    }
    pollingRef.current = false
    setStep('idle')
    setErrorMessage('')
    setAuthResponse(null)
    setPollStatus('')
    clientIdRef.current = ''
    clientSecretRef.current = ''
    deviceCodeRef.current = ''
    pollIntervalRef.current = 2000
  }, [])

  useEffect(() => {
    if (!open) resetState()
  }, [open, resetState])

  const stopPolling = useCallback(() => {
    pollingRef.current = false
    if (pollTimerRef.current) {
      clearTimeout(pollTimerRef.current)
      pollTimerRef.current = null
    }
  }, [])

  const doPoll = useCallback(async () => {
    if (!pollingRef.current) return
    const cid = clientIdRef.current
    const cs = clientSecretRef.current
    const dc = deviceCodeRef.current
    if (!cid || !cs || !dc) return

    try {
      const resp = await deviceFlowPoll({
        clientId: cid,
        clientSecret: cs,
        deviceCode: dc,
      })

      if (resp.refreshToken) {
        stopPolling()
        setStep('success')
        setPollStatus('授权成功！正在添加凭证...')

        try {
          const lt = loginTypeRef.current
          const authMethod = (lt === 'enterprise' || lt === 'personal') ? 'idc' : 'social'
          await addCredential({
            refreshToken: resp.refreshToken,
            authMethod,
            clientId: authMethod === 'idc' ? cid : undefined,
            clientSecret: authMethod === 'idc' ? cs : undefined,
          })
          toast.success('设备登录成功，凭证已自动添加')
          queryClient.invalidateQueries({ queryKey: ['credentials'] })
          onOpenChange(false)
        } catch (addErr) {
          toast.error(`凭证添加失败: ${extractErrorMessage(addErr)}`)
          setStep('error')
          setErrorMessage(`授权成功但添加凭证失败: ${extractErrorMessage(addErr)}`)
        }
        return
      }

      if (resp.error === 'authorization_pending') {
        setPollStatus('等待授权... 请在浏览器中完成授权')
        return
      }

      if (resp.error === 'slow_down') {
        pollIntervalRef.current = Math.min(pollIntervalRef.current + 2000, 10000)
        setPollStatus(`服务端要求降速，轮询间隔 ${pollIntervalRef.current / 1000}s`)
        return
      }

      if (resp.error === 'expired_token') {
        stopPolling()
        setStep('error')
        setErrorMessage('设备码已过期，请重新登录')
        return
      }

      if (resp.error) {
        stopPolling()
        setStep('error')
        setErrorMessage(resp.errorDescription || resp.error)
        return
      }

      setPollStatus('等待响应...')
    } catch (err) {
      stopPolling()
      setStep('error')
      setErrorMessage(`轮询失败: ${extractErrorMessage(err)}`)
    }
  }, [onOpenChange, stopPolling])

  const scheduleNextPoll = useCallback(() => {
    if (!pollingRef.current) return
    if (pollTimerRef.current) clearTimeout(pollTimerRef.current)
    pollTimerRef.current = setTimeout(async () => {
      if (!pollingRef.current) return
      await doPoll()
      scheduleNextPoll()
    }, pollIntervalRef.current)
  }, [doPoll])

  const handleLogin = async () => {
    setStep('registering')
    setErrorMessage('')

    try {
      const reg = await deviceFlowRegister({
        loginType,
        enterpriseStartUrl: loginType === 'enterprise' ? enterpriseStartUrl : undefined,
      })
      // 立即写入 ref（不依赖 setState 的异步更新）
      clientIdRef.current = reg.clientId
      clientSecretRef.current = reg.clientSecret

      setStep('authorizing')
      const auth = await deviceFlowAuthorize({
        clientId: reg.clientId,
        clientSecret: reg.clientSecret,
        loginType,
        enterpriseStartUrl: loginType === 'enterprise' ? enterpriseStartUrl : undefined,
      })

      setAuthResponse(auth)
      deviceCodeRef.current = auth.deviceCode
      pollIntervalRef.current = Math.max(2000, auth.interval * 1000)

      setStep('polling')
      pollingRef.current = true
      setPollStatus('轮询中... 请在浏览器中完成授权')

      // 首次 poll 立即执行，然后调度后续
      doPoll().then(() => {
        scheduleNextPoll()
      })
    } catch (err) {
      setStep('error')
      setErrorMessage(extractErrorMessage(err))
    }
  }

  const handleCopyLink = async () => {
    if (authResponse?.verificationUriComplete) {
      await navigator.clipboard.writeText(authResponse.verificationUriComplete)
      toast.success('验证链接已复制')
    }
  }

  const handleOpenLink = () => {
    if (authResponse?.verificationUriComplete) {
      window.open(authResponse.verificationUriComplete, '_blank', 'noopener,noreferrer')
    }
  }

  const handleCopyUserCode = async () => {
    if (authResponse?.userCode) {
      await navigator.clipboard.writeText(authResponse.userCode)
      toast.success('User Code 已复制')
    }
  }

  const isBusy = step === 'registering' || step === 'authorizing' || step === 'polling'

  return (
    <Dialog open={open} onOpenChange={(v) => { if (!isBusy) onOpenChange(v) }}>
      <DialogContent className="sm:max-w-lg max-h-[85vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>设备登录</DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-4 overflow-y-auto flex-1 pr-1">
          {/* 登录类型选择 */}
          <div className="space-y-2">
            <label className="text-sm font-medium">登录类型</label>
            <div className="flex gap-2">
              {(['social', 'personal', 'enterprise'] as const).map((type) => (
                <Button
                  key={type}
                  size="sm"
                  variant={loginType === type ? 'default' : 'outline'}
                  onClick={() => setLoginType(type)}
                  disabled={isBusy}
                >
                  {type === 'social' ? 'Social' : type === 'personal' ? 'Personal' : 'Enterprise'}
                </Button>
              ))}
            </div>
          </div>

          {/* Enterprise URL */}
          {loginType === 'enterprise' && (
            <div className="space-y-2">
              <label className="text-sm font-medium">Enterprise Start URL</label>
              <Input
                placeholder="https://d-xxxxxxxxxx.awsapps.com/start"
                value={enterpriseStartUrl}
                onChange={(e) => setEnterpriseStartUrl(e.target.value)}
                disabled={isBusy}
              />
            </div>
          )}

          {/* 步骤状态 */}
          {step === 'registering' && (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-primary" />
              正在注册 OIDC 客户端...
            </div>
          )}

          {step === 'authorizing' && (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-primary" />
              正在获取设备授权...
            </div>
          )}

          {step === 'polling' && authResponse && (
            <>
              <div className="space-y-3">
                <div className="space-y-2">
                  <label className="text-sm font-medium">验证链接</label>
                  <div className="flex gap-2">
                    <Input
                      readOnly
                      value={authResponse.verificationUriComplete}
                      className="text-xs font-mono"
                    />
                    <Button size="sm" variant="outline" onClick={handleCopyLink}>
                      复制
                    </Button>
                    <Button size="sm" variant="outline" onClick={handleOpenLink}>
                      打开
                    </Button>
                  </div>
                </div>

                <div className="space-y-2">
                  <label className="text-sm font-medium">User Code</label>
                  <div className="flex gap-2 items-center">
                    <span className="text-lg font-mono font-bold tracking-wider">{authResponse.userCode}</span>
                    <Button size="sm" variant="outline" onClick={handleCopyUserCode}>
                      复制
                    </Button>
                  </div>
                </div>

                <div className="flex items-center gap-2 text-sm text-yellow-600 dark:text-yellow-400">
                  <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-yellow-500" />
                  {pollStatus}
                </div>
              </div>
            </>
          )}

          {step === 'success' && (
            <div className="flex items-center gap-2 text-sm text-green-600 dark:text-green-400">
              {pollStatus}
            </div>
          )}

          {step === 'error' && (
            <div className="text-sm text-red-600 dark:text-red-400">
              {errorMessage}
            </div>
          )}

          {step === 'idle' && (
            <div className="text-sm text-muted-foreground">
              点击"登录"按钮，将通过 AWS OIDC Device Flow 自动获取凭证。
              <br />
              Social / Personal 登录需要浏览器完成授权。
            </div>
          )}
        </div>

        <DialogFooter>
          {step === 'polling' && (
            <Button
              variant="outline"
              onClick={() => {
                stopPolling()
                setStep('idle')
                setPollStatus('')
              }}
            >
              取消轮询
            </Button>
          )}
          {step === 'error' && (
            <Button variant="outline" onClick={resetState}>
              重试
            </Button>
          )}
          {(step === 'idle' || step === 'error') && (
            <Button onClick={handleLogin} disabled={isBusy}>
              登录
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
