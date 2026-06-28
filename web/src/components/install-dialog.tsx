import { useEffect, useMemo, useState } from "react"
import {
  Check,
  Copy,
  ExternalLink,
  Plug,
  Terminal,
} from "lucide-react"
import { Badge } from "@/components/ui/badge"
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
import { cn } from "@/lib/utils"
import {
  type Agent,
  type InstallParams,
  type Target,
  buildMcpAddCommand,
  buildMcpServersJson,
  buildOneLiner,
  fetchSkillMarkdown,
} from "@/lib/install"

type Mode = "auto" | "manual"
type Tab = "skill" | "mcp" | "json"

interface InstallDialogProps {
  /** Skill/MCP instance name from /api/health. Always set after first load. */
  instanceName: string
  /** Indexed root from /api/health. null while health is still loading. */
  basePath: string | null
}

const AGENTS: { value: Agent; label: string; hint: string }[] = [
  { value: "opencode", label: "opencode", hint: "~/.agents/skills" },
  { value: "claude", label: "Claude Code", hint: "~/.claude/skills" },
  { value: "codex", label: "Codex", hint: "~/.agents/skills" },
  { value: "generic", label: "通用", hint: "仅给端点" },
]

const TARGETS: { value: Target; label: string }[] = [
  { value: "global", label: "全局" },
  { value: "project", label: "当前项目" },
]

const MODES: { value: Mode; label: string }[] = [
  { value: "auto", label: "一键安装" },
  { value: "manual", label: "手动安装" },
]

const TABS: { value: Tab; label: string }[] = [
  { value: "skill", label: "SKILL.md" },
  { value: "mcp", label: "MCP 命令" },
  { value: "json", label: "mcpServers JSON" },
]

export function InstallDialog({ instanceName, basePath }: InstallDialogProps) {
  const [open, setOpen] = useState(false)
  const [mode, setMode] = useState<Mode>("auto")
  const [agent, setAgent] = useState<Agent>("opencode")
  const [target, setTarget] = useState<Target>("global")
  const [tab, setTab] = useState<Tab>("skill")
  const [copied, setCopied] = useState<string | null>(null)
  const [skillMd, setSkillMd] = useState<string>("")
  const [origin, setOrigin] = useState("")

  const ready = basePath !== null
  const url = origin ? `${origin}/mcp` : ""
  const scope = basePath ?? ""

  // Resolve origin once the dialog mounts ( SSR-safe no-op ).
  useEffect(() => {
    if (!open) return
    if (typeof window === "undefined") return
    setOrigin(window.location.origin)
  }, [open])

  const params: InstallParams = useMemo(
    () => ({
      name: instanceName,
      url,
      scope,
      notes: "",
      agent,
      target,
    }),
    [instanceName, url, scope, agent, target],
  )

  // Live-preview SKILL.md only when the user is looking at it.
  useEffect(() => {
    if (!open || mode !== "manual" || tab !== "skill" || !ready) {
      setSkillMd("")
      return
    }
    const ctrl = new AbortController()
    fetchSkillMarkdown(origin, params, ctrl.signal).then((md) =>
      setSkillMd(md ?? ""),
    )
    return () => ctrl.abort()
  }, [open, mode, tab, ready, origin, params])

  const oneLiner = useMemo(
    () => (ready && origin ? buildOneLiner(origin, params) : ""),
    [ready, origin, params],
  )
  const mcpCmd = useMemo(
    () => (ready ? buildMcpAddCommand(params) : ""),
    [ready, params],
  )
  const jsonSnippet = useMemo(
    () => (ready ? buildMcpServersJson(params) : ""),
    [ready, params],
  )

  const sharesAgentsPath = agent === "opencode" || agent === "codex"

  const copy = async (key: string, text: string) => {
    if (!text) return
    try {
      await navigator.clipboard.writeText(text)
      setCopied(key)
      setTimeout(() => setCopied((c) => (c === key ? null : c)), 1500)
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
              <code className="font-mono break-all">{url || "…"}</code>
              <span className="text-muted-foreground">索引范围</span>
              {ready ? (
                <code className="font-mono break-all">{basePath}</code>
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
            <span className="text-sm font-medium">安装方式</span>
            <div className="flex gap-1 rounded-md bg-muted/40 p-1">
              {MODES.map((m) => (
                <button
                  key={m.value}
                  type="button"
                  onClick={() => setMode(m.value)}
                  className={cn(
                    "flex-1 rounded-[inherit] px-3 py-1.5 text-sm font-medium transition-colors",
                    mode === m.value
                      ? "bg-background text-foreground shadow-sm"
                      : "text-muted-foreground hover:text-foreground",
                  )}
                >
                  {m.label}
                </button>
              ))}
            </div>
          </section>

          {mode === "auto" ? (
            <section className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">一键安装</span>
                <Badge variant="secondary" className="font-normal">
                  <Terminal className="size-3" />
                  curl | bash
                </Badge>
              </div>
              <pre
                className={cn(
                  "overflow-x-auto whitespace-pre-wrap break-all rounded-md border bg-muted/40 p-3 font-mono text-xs leading-relaxed",
                  !ready && "opacity-50",
                )}
              >
                {ready ? oneLiner : "正在获取实例信息…"}
              </pre>
              {sharesAgentsPath && (
                <p className="text-muted-foreground text-xs">
                  skill 装到{" "}
                  <code className="font-mono">~/.agents/skills</code>，opencode
                  与 Codex 共享此目录。
                </p>
              )}
              <div className="flex justify-end">
                <Button
                  size="sm"
                  className="gap-1.5"
                  disabled={!ready}
                  onClick={() => copy("liner", oneLiner)}
                >
                  {copied === "liner" ? (
                    <>
                      <Check className="size-3.5" />
                      已复制
                    </>
                  ) : (
                    <>
                      <Copy className="size-3.5" />
                      复制命令
                    </>
                  )}
                </Button>
              </div>
            </section>
          ) : (
            <section className="space-y-2">
              <span className="text-sm font-medium">手动安装</span>
              <div className="flex gap-1">
                {TABS.map((t) => (
                  <button
                    key={t.value}
                    type="button"
                    onClick={() => setTab(t.value)}
                    className={cn(
                      "rounded-md px-2 py-1 text-xs transition-colors",
                      tab === t.value
                        ? "bg-secondary text-secondary-foreground"
                        : "text-muted-foreground hover:text-foreground",
                    )}
                  >
                    {t.label}
                  </button>
                ))}
              </div>
              <SnippetBlock
                value={
                  tab === "skill"
                    ? skillMd
                    : tab === "mcp"
                      ? mcpCmd
                      : jsonSnippet
                }
                copied={copied === tab}
                onCopy={() =>
                  copy(
                    tab,
                    tab === "skill"
                      ? skillMd
                      : tab === "mcp"
                        ? mcpCmd
                        : jsonSnippet,
                  )
                }
              />
            </section>
          )}

          <div className="flex justify-end">
            <Button
              asChild
              variant="link"
              size="sm"
              className="h-auto gap-1.5 px-0"
            >
              <a
                href="https://opencode.ai/docs/skills"
                target="_blank"
                rel="noreferrer"
              >
                <ExternalLink className="size-3.5" />
                Skill 文档
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
