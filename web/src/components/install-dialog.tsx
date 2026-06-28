import { useEffect, useMemo, useState } from "react"
import { Check, Copy, ExternalLink, Plug } from "lucide-react"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog"
import { Skeleton } from "@/components/ui/skeleton"
import { withPrefix } from "@/lib/config"
import { cn } from "@/lib/utils"
import {
  type Agent,
  type InstallParams,
  type Target,
  buildMcpAddCommand,
  buildMcpServersJson,
} from "@/lib/install"

type Tab = "mcp" | "json"

interface InstallDialogProps {
  /** Skill/MCP instance name from /api/health. Always set after first load. */
  instanceName: string
  /** Indexed root from /api/health. null while health is still loading. */
  basePath: string | null
}

const AGENTS: { value: Agent; label: string; hint: string }[] = [
  { value: "opencode", label: "opencode", hint: "原生 mcp add" },
  { value: "claude", label: "Claude Code", hint: "--transport http" },
  { value: "codex", label: "Codex", hint: "原生 mcp add" },
  { value: "generic", label: "通用", hint: "仅给端点" },
]

const TARGETS: { value: Target; label: string }[] = [
  { value: "global", label: "全局" },
  { value: "project", label: "当前项目" },
]

const TABS: { value: Tab; label: string }[] = [
  { value: "mcp", label: "MCP 命令" },
  { value: "json", label: "mcpServers JSON" },
]

export function InstallDialog({ instanceName, basePath }: InstallDialogProps) {
  const [open, setOpen] = useState(false)
  const [agent, setAgent] = useState<Agent>("opencode")
  const [target, setTarget] = useState<Target>("global")
  const [tab, setTab] = useState<Tab>("mcp")
  const [copied, setCopied] = useState(false)
  const [origin, setOrigin] = useState("")

  const ready = basePath !== null
  const url = origin ? `${origin}${withPrefix("/mcp")}` : ""

  useEffect(() => {
    if (!open) return
    if (typeof window === "undefined") return
    setOrigin(window.location.origin)
  }, [open])

  const params: InstallParams = useMemo(
    () => ({ name: instanceName, url, agent, target }),
    [instanceName, url, agent, target],
  )

  const mcpCmd = useMemo(
    () => (ready ? buildMcpAddCommand(params) : ""),
    [ready, params],
  )
  const jsonSnippet = useMemo(
    () => (ready ? buildMcpServersJson(params) : ""),
    [ready, params],
  )

  const current = tab === "mcp" ? mcpCmd : jsonSnippet

  const copy = async (text: string) => {
    if (!text) return
    try {
      await navigator.clipboard.writeText(text)
      setCopied(true)
      setTimeout(() => setCopied(false), 1500)
    } catch {
      // clipboard unavailable
    }
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button variant="outline" size="sm" className="gap-2">
          <Plug className="size-4" />
          接入 Agent
        </Button>
      </DialogTrigger>
      <DialogContent className="max-w-xl sm:max-w-xl max-h-[calc(100dvh-2rem)] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>接入到 Agent</DialogTitle>
          <DialogDescription>
            把本实例的文件搜索能力装进 opencode / Claude Code / Codex。
          </DialogDescription>
        </DialogHeader>

        <div className="min-w-0 space-y-4">
          <section className="space-y-2">
            <span className="text-sm font-medium">实例信息</span>
            <div className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1.5 rounded-md border bg-muted/30 p-3 text-xs">
              <span className="text-muted-foreground">名称</span>
              <code className="font-mono">{instanceName || "plocate"}</code>
              <span className="text-muted-foreground">MCP 端点</span>
              <code className="min-w-0 break-all font-mono">{url || "…"}</code>
              <span className="text-muted-foreground">索引范围</span>
              {ready ? (
                <code className="min-w-0 break-all font-mono">{basePath}</code>
              ) : (
                <Skeleton className="h-4 w-40" />
              )}
            </div>
          </section>

          <section className="space-y-2">
            <span className="text-sm font-medium">安装目标</span>
            <div className="grid grid-cols-2 gap-3">
              <RadioGroup
                label="Agent"
                value={agent}
                options={AGENTS.map((a) => ({
                  value: a.value,
                  label: a.label,
                  hint: a.hint,
                }))}
                onChange={(v) => setAgent(v as Agent)}
              />
              <RadioGroup
                label="安装到"
                value={target}
                options={TARGETS.map((t) => ({ value: t.value, label: t.label }))}
                onChange={(v) => setTarget(v as Target)}
              />
            </div>
          </section>

          <section className="space-y-2">
            <div className="flex gap-1 rounded-md bg-muted/40 p-1">
              {TABS.map((t) => (
                <button
                  key={t.value}
                  type="button"
                  onClick={() => setTab(t.value)}
                  className={cn(
                    "flex-1 rounded-[inherit] px-3 py-1.5 text-sm font-medium transition-colors",
                    tab === t.value
                      ? "bg-background text-foreground shadow-sm"
                      : "text-muted-foreground hover:text-foreground",
                  )}
                >
                  {t.label}
                </button>
              ))}
            </div>
            <SnippetBlock
              value={current}
              copied={copied}
              onCopy={() => copy(current)}
            />
            <p className="text-muted-foreground text-xs">
              复制后粘贴到终端执行（MCP 命令）或合并进对应配置文件（JSON）。
            </p>
          </section>

          <div className="flex justify-end">
            <Button
              asChild
              variant="link"
              size="sm"
              className="h-auto gap-1.5 px-0"
            >
              <a
                href="https://modelcontextprotocol.io"
                target="_blank"
                rel="noreferrer"
              >
                <ExternalLink className="size-3.5" />
                MCP 文档
              </a>
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}

function RadioGroup({
  label,
  value,
  options,
  onChange,
}: {
  label: string
  value: string
  options: { value: string; label: string; hint?: string }[]
  onChange: (v: string) => void
}) {
  return (
    <div className="space-y-1.5">
      <span className="text-muted-foreground text-xs">{label}</span>
      <div className="space-y-1">
        {options.map((o) => (
          <label
            key={o.value}
            className="flex cursor-pointer items-start gap-2 rounded-md border border-transparent p-1.5 has-[:checked]:border-border has-[:checked]:bg-muted/40"
          >
            <input
              type="radio"
              name={label}
              checked={value === o.value}
              onChange={() => onChange(o.value)}
              className="mt-0.5 size-3.5 accent-primary"
            />
            <span className="flex flex-col">
              <span className="text-sm leading-tight">{o.label}</span>
              {o.hint && (
                <span className="text-muted-foreground text-xs">{o.hint}</span>
              )}
            </span>
          </label>
        ))}
      </div>
    </div>
  )
}

function SnippetBlock({
  value,
  copied,
  onCopy,
}: {
  value: string
  copied: boolean
  onCopy: () => void
}) {
  return (
    <div className="relative min-w-0">
      <pre className="max-h-60 min-w-0 overflow-auto whitespace-pre rounded-md border bg-muted/40 p-3 pr-10 font-mono text-xs leading-relaxed">
        {value || "（等待生成）"}
      </pre>
      <Button
        type="button"
        variant="ghost"
        size="icon"
        className="absolute top-1.5 right-1.5 size-7"
        disabled={!value}
        onClick={onCopy}
      >
        {copied ? (
          <Check className="size-3.5 text-emerald-600" />
        ) : (
          <Copy className="size-3.5" />
        )}
      </Button>
    </div>
  )
}
