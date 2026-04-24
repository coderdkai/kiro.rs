import { useState } from 'react'
import { toast } from 'sonner'
import {
  MoreHorizontal,
  RefreshCw,
  ChevronUp,
  ChevronDown,
  Wallet,
  Trash2,
  RotateCcw,
  Loader2,
} from 'lucide-react'
import {
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from '@/components/ui/table'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Switch } from '@/components/ui/switch'
import { Checkbox } from '@/components/ui/checkbox'
import { Input } from '@/components/ui/input'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
} from '@/components/ui/dropdown-menu'
import type { CredentialStatusItem, BalanceResponse } from '@/types/api'
import {
  useSetDisabled,
  useSetPriority,
  useResetFailure,
  useDeleteCredential,
  useForceRefreshToken,
} from '@/hooks/use-credentials'

interface CredentialTableProps {
  credentials: CredentialStatusItem[]
  selectedIds: Set<number>
  onToggleSelect: (id: number) => void
  onSelectAll: () => void
  onViewBalance: (id: number) => void
  balanceMap: Map<number, BalanceResponse>
  loadingBalanceIds: Set<number>
}

function formatLastUsed(lastUsedAt: string | null): string {
  if (!lastUsedAt) return '-'
  const date = new Date(lastUsedAt)
  const now = new Date()
  const diff = now.getTime() - date.getTime()
  if (diff < 0) return '刚刚'
  const seconds = Math.floor(diff / 1000)
  if (seconds < 60) return `${seconds}s`
  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return `${minutes}m`
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `${hours}h`
  const days = Math.floor(hours / 24)
  return `${days}d`
}

function formatUsage(balance: BalanceResponse | null | undefined, loading: boolean): string {
  if (loading) return '...'
  if (!balance) return '-'
  return `${balance.remaining.toFixed(0)}/${balance.usageLimit.toFixed(0)}`
}

function CredentialRow({
  credential,
  selected,
  onToggleSelect,
  onViewBalance,
  balance,
  loadingBalance,
}: {
  credential: CredentialStatusItem
  selected: boolean
  onToggleSelect: () => void
  onViewBalance: (id: number) => void
  balance: BalanceResponse | null
  loadingBalance: boolean
}) {
  const [editingPriority, setEditingPriority] = useState(false)
  const [priorityValue, setPriorityValue] = useState(String(credential.priority))
  const [showDeleteDialog, setShowDeleteDialog] = useState(false)

  const setDisabled = useSetDisabled()
  const setPriority = useSetPriority()
  const resetFailure = useResetFailure()
  const deleteCredential = useDeleteCredential()
  const forceRefresh = useForceRefreshToken()

  const handleToggleDisabled = () => {
    setDisabled.mutate(
      { id: credential.id, disabled: !credential.disabled },
      {
        onSuccess: (res) => toast.success(res.message),
        onError: (err) => toast.error('操作失败: ' + (err as Error).message),
      }
    )
  }

  const handlePrioritySave = () => {
    const val = parseInt(priorityValue, 10)
    if (isNaN(val) || val < 0) {
      toast.error('优先级必须是非负整数')
      return
    }
    setPriority.mutate(
      { id: credential.id, priority: val },
      {
        onSuccess: (res) => {
          toast.success(res.message)
          setEditingPriority(false)
        },
        onError: (err) => toast.error('操作失败: ' + (err as Error).message),
      }
    )
  }

  const handleDelete = () => {
    if (!credential.disabled) {
      toast.error('请先禁用凭据再删除')
      setShowDeleteDialog(false)
      return
    }
    deleteCredential.mutate(credential.id, {
      onSuccess: (res) => {
        toast.success(res.message)
        setShowDeleteDialog(false)
      },
      onError: (err) => toast.error('删除失败: ' + (err as Error).message),
    })
  }

  const authLabel =
    credential.authMethod === 'api_key'
      ? 'Key'
      : credential.authMethod === 'idc'
        ? 'IdC'
        : credential.authMethod === 'social'
          ? 'Social'
          : credential.authMethod || '-'

  return (
    <>
      <TableRow
        className={
          credential.disabled
            ? 'bg-destructive/5 opacity-70'
            : credential.isCurrent
              ? 'bg-primary/5'
              : ''
        }
        data-state={selected ? 'selected' : undefined}
      >
        <TableCell>
          <Checkbox checked={selected} onCheckedChange={onToggleSelect} />
        </TableCell>
        <TableCell className="font-medium">
          <div className="flex items-center gap-1.5 whitespace-nowrap">
            <span>#{credential.id}</span>
            {credential.isCurrent && (
              <Badge variant="success" className="text-[10px] px-1.5 py-0 shrink-0">活跃</Badge>
            )}
            {credential.disabled && (
              <Badge variant="destructive" className="text-[10px] px-1.5 py-0 shrink-0">禁用</Badge>
            )}
          </div>
        </TableCell>
        <TableCell>
          {credential.email ? (
            <span className="truncate max-w-[200px] inline-block" title={credential.email}>
              {credential.email}
            </span>
          ) : (
            <span className="text-muted-foreground text-xs">凭据 #{credential.id}</span>
          )}
        </TableCell>
        <TableCell>
          <div className="flex items-center gap-1">
            <Badge variant="secondary" className="text-[10px] px-1.5 py-0">{authLabel}</Badge>
            {credential.disabled && credential.disabledReason && (
              <Badge variant="outline" className="text-[10px] px-1 py-0 text-destructive border-destructive/30">
                {credential.disabledReason}
              </Badge>
            )}
          </div>
        </TableCell>
        <TableCell>
          <Switch
            checked={!credential.disabled}
            onCheckedChange={handleToggleDisabled}
            disabled={setDisabled.isPending}
          />
        </TableCell>
        <TableCell>
          {editingPriority ? (
            <div className="flex items-center gap-1">
              <Input
                type="number"
                value={priorityValue}
                onChange={(e) => setPriorityValue(e.target.value)}
                className="w-14 h-6 text-xs px-1"
                min="0"
                onKeyDown={(e) => {
                  if (e.key === 'Enter') handlePrioritySave()
                  if (e.key === 'Escape') {
                    setEditingPriority(false)
                    setPriorityValue(String(credential.priority))
                  }
                }}
                autoFocus
              />
              <Button size="sm" variant="ghost" className="h-6 w-6 p-0 text-xs" onClick={handlePrioritySave} disabled={setPriority.isPending}>
                ✓
              </Button>
            </div>
          ) : (
            <span
              className="cursor-pointer hover:underline"
              onClick={() => setEditingPriority(true)}
            >
              {credential.priority}
            </span>
          )}
        </TableCell>
        <TableCell>
          <span className={credential.failureCount > 0 ? 'text-red-500 font-medium' : ''}>
            {credential.failureCount}
          </span>
          {credential.refreshFailureCount > 0 && (
            <span className="text-red-400 text-xs ml-1" title="刷新失败次数">
              (R:{credential.refreshFailureCount})
            </span>
          )}
        </TableCell>
        <TableCell className="text-xs">{credential.successCount}</TableCell>
        <TableCell>
          <span className="text-xs">
            {loadingBalance ? (
              <Loader2 className="inline w-3 h-3 animate-spin" />
            ) : (
              formatUsage(balance, false)
            )}
          </span>
        </TableCell>
        <TableCell className="text-xs text-muted-foreground">{formatLastUsed(credential.lastUsedAt)}</TableCell>
        <TableCell>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="sm" className="h-7 w-7 p-0">
                <MoreHorizontal className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onClick={() => onViewBalance(credential.id)}>
                <Wallet className="h-4 w-4" />
                查看余额
              </DropdownMenuItem>
              <DropdownMenuItem
                onClick={() => {
                  resetFailure.mutate(credential.id, {
                    onSuccess: (res) => toast.success(res.message),
                    onError: (err) => toast.error('操作失败: ' + (err as Error).message),
                  })
                }}
                disabled={credential.failureCount === 0 && credential.refreshFailureCount === 0}
              >
                <RotateCcw className="h-4 w-4" />
                重置失败
              </DropdownMenuItem>
              <DropdownMenuItem
                onClick={() => {
                  forceRefresh.mutate(credential.id, {
                    onSuccess: (res) => toast.success(res.message),
                    onError: (err) => toast.error('刷新失败: ' + (err as Error).message),
                  })
                }}
                disabled={credential.disabled || credential.authMethod === 'api_key'}
              >
                <RefreshCw className="h-4 w-4" />
                刷新 Token
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                onClick={() => {
                  setPriority.mutate(
                    { id: credential.id, priority: Math.max(0, credential.priority - 1) },
                    {
                      onSuccess: (res) => toast.success(res.message),
                      onError: (err) => toast.error('操作失败: ' + (err as Error).message),
                    }
                  )
                }}
                disabled={credential.priority === 0}
              >
                <ChevronUp className="h-4 w-4" />
                提高优先级
              </DropdownMenuItem>
              <DropdownMenuItem
                onClick={() => {
                  setPriority.mutate(
                    { id: credential.id, priority: credential.priority + 1 },
                    {
                      onSuccess: (res) => toast.success(res.message),
                      onError: (err) => toast.error('操作失败: ' + (err as Error).message),
                    }
                  )
                }}
              >
                <ChevronDown className="h-4 w-4" />
                降低优先级
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                onClick={() => setShowDeleteDialog(true)}
                disabled={!credential.disabled}
                className="text-destructive focus:text-destructive"
              >
                <Trash2 className="h-4 w-4" />
                删除
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </TableCell>
      </TableRow>

      <Dialog open={showDeleteDialog} onOpenChange={setShowDeleteDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>确认删除凭据</DialogTitle>
            <DialogDescription>
              确定要删除凭据 #{credential.id}{credential.email ? ` (${credential.email})` : ''} 吗？此操作无法撤销。
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowDeleteDialog(false)} disabled={deleteCredential.isPending}>
              取消
            </Button>
            <Button variant="destructive" onClick={handleDelete} disabled={deleteCredential.isPending || !credential.disabled}>
              确认删除
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}

export function CredentialTable({
  credentials,
  selectedIds,
  onToggleSelect,
  onSelectAll,
  onViewBalance,
  balanceMap,
  loadingBalanceIds,
}: CredentialTableProps) {
  const allSelected = credentials.length > 0 && credentials.every((c) => selectedIds.has(c.id))
  const someSelected = credentials.some((c) => selectedIds.has(c.id))

  return (
    <div className="rounded-md border">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead className="w-[40px]">
              <Checkbox
                checked={allSelected}
                onCheckedChange={onSelectAll}
                aria-label="全选"
                {...(someSelected && !allSelected ? { 'data-state': 'indeterminate' as const } : {})}
              />
            </TableHead>
            <TableHead className="w-[120px]">ID</TableHead>
            <TableHead>邮箱</TableHead>
            <TableHead className="w-[60px]">类型</TableHead>
            <TableHead className="w-[60px]">启用</TableHead>
            <TableHead className="w-[70px]">优先级</TableHead>
            <TableHead className="w-[80px]">失败</TableHead>
            <TableHead className="w-[60px]">成功</TableHead>
            <TableHead className="w-[90px]">剩余用量</TableHead>
            <TableHead className="w-[60px]">最近</TableHead>
            <TableHead className="w-[40px]"></TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {credentials.length === 0 ? (
            <TableRow>
              <TableCell colSpan={11} className="h-24 text-center text-muted-foreground">
                暂无凭据
              </TableCell>
            </TableRow>
          ) : (
            credentials.map((credential) => (
              <CredentialRow
                key={credential.id}
                credential={credential}
                selected={selectedIds.has(credential.id)}
                onToggleSelect={() => onToggleSelect(credential.id)}
                onViewBalance={onViewBalance}
                balance={balanceMap.get(credential.id) || null}
                loadingBalance={loadingBalanceIds.has(credential.id)}
              />
            ))
          )}
        </TableBody>
      </Table>
    </div>
  )
}
