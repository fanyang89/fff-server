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
import { Input } from "@/components/ui/input"
import { Textarea } from "@/components/ui/textarea"
import { cn } from "@/lib/utils"
import {
  type Agent,
  type InstallParams,
  type Target,
  buildMcpAddCommand,
  buildMcpServersJson,
  buildOneLiner,
  defaultInstanceUrl,
  fetchSkillMarkdown,
  validateInstanceUrl,
  validateSkillName,
} from "@/lib/install"

type Tab = "skill" | "mcp" | "json"

const AGENTS: { value: Agent; label: string; hint: string }[] = [
  { value: "opencode", label: "opencode", hint: "原生 skill + mcp add" },
  { value: "claude", label: "Claude Code", hint: ".claude/skills 兼容" },
  { value: "generic", label: "通用", hint: "仅给端点信息" },
]

const TARGETS: { value: Target; label: string }[] = [
  { value: "global", label: "全局" },
  { value: "project", label: "当前项目" },
]

const TABS: { value: Tab; label: string }[] = [
  { value: "skill", label: "SKILL.md" },
  { value: "mcp", label: "MCP 命令" },
  { value: "json", label: "mcpServers JSON" },
]

export function InstallDialog() {
  const [open, setOpen] = useState(false)
  const [params, setParams] = useState<InstallParams>(() => ({
    name: "plocate",
    url: defaultInstanceUrl(),
    scope: "",
    notes: "",
    agent: "opencode",
    target: "global",
  }))
  const [tab, setTab] = useState<Tab>("skill")
  const [copied, setCopied] = useState<string | null>(null)
  const [skillMd, setSkillMd] = useState<string>("")
  const [origin, setOrigin] = useState("")

  const nameOk = validateSkillName(params.name)
  const urlOk = validateInstanceUrl(params.url)
  const formOk = nameOk && urlOk

  useEffect(() => {
    if (!open) return
    if (typeof window === "undefined") return
    setOrigin(window.location.origin)
  }, [open])

  // Live-preview SKILL.md when on the skill tab and the form is valid.
  useEffect(() => {
    if (!open || tab !== "skill" || !formOk) {
      setSkillMd("")
      return
    }
    const ctrl = new AbortController()
    fetchSkillMarkdown(origin, params, ctrl.signal).then((md) =>
      setSkillMd(md ?? ""),
    )
    return () => ctrl.abort()
  }, [open, tab, formOk, origin, params])

  const oneLiner = useMemo(
    () => (formOk && origin ? buildOneLiner(origin, params) : ""),
    [formOk, origin, params],
  )
  const mcpCmd = useMemo(
    () => (formOk ? buildMcpAddCommand(params) : ""),
    [formOk, params],
  )
  const jsonSnippet = useMemo(
    () => (formOk ? buildMcpServersJson(params) : ""),
    [formOk, params],
  )

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

  const set = <K extends keyof InstallParams>(key: K, value: InstallParams[K]) =>
    setParams((p) => ({ ...p, [key]: value }))

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button variant="outline" size="sm" className="gap-2">
          <Plug className="size-4" />
          接入 Agent
        </Button>
      </DialogTrigger>
      <DialogContent className="max-w-xl">
        <DialogHeader>
          <DialogTitle>接入到 Agent</DialogTitle>
          <DialogDescription>
            把本实例的文件搜索能力装进你的 opencode / Claude Code。
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <FormGrid>
            <Field
              label="实例名"
              hint="同时是 skill 目录名与 MCP 名"
              error={params.name && !nameOk ? "仅 a-z 0-9 -，不可 - 开头/结尾" : ""}
            >
              <Input
                value={params.name}
                onChange={(e) => set("name", e.target.value)}
                placeholder="plocate"
                spellCheck={false}
                autoComplete="off"
                aria-invalid={!nameOk}
              />
            </Field>

            <Field label="MCP 端点" hint="通常为本服务的 /mcp">
              <Input
                value={params.url}
                onChange={(e) => set("url", e.target.value)}
                placeholder="https://host/mcp"
                spellCheck={false}
                autoComplete="off"
                aria-invalid={!urlOk}
              />
            </Field>

            <Field label="索引范围" hint="人类可读，告诉 agent 索引覆盖了什么">
              <Input
                value={params.scope}
                onChange={(e) => set("scope", e.target.value)}
                placeholder="/srv/files"
                spellCheck={false}
                autoComplete="off"
              />
            </Field>

            <Field label="备注" hint="可选，写入 SKILL.md 末尾">
              <Textarea
                value={params.notes}
                onChange={(e) => set("notes", e.target.value)}
                placeholder="例如：仅索引 /srv，不含 /home"
                rows={2}
                className="resize-none font-normal"
              />
            </Field>
          </FormGrid>

          <section className="space-y-2">
            <span className="text-sm font-medium">目标</span>
            <div className="grid grid-cols-2 gap-3">
              <RadioGroup
                label="Agent"
                value={params.agent}
                options={AGENTS.map((a) => ({
                  value: a.value,
                  label: a.label,
                  hint: a.hint,
                }))}
                onChange={(v) => set("agent", v as Agent)}
              />
              <RadioGroup
                label="安装到"
                value={params.target}
                options={TARGETS.map((t) => ({ value: t.value, label: t.label }))}
                onChange={(v) => set("target", v as Target)}
              />
            </div>
          </section>

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
                !formOk && "opacity-50",
              )}
            >
              {oneLiner || "补全实例名与端点后生成命令"}
            </pre>
            <div className="flex justify-end">
              <Button
                size="sm"
                className="gap-1.5"
                disabled={!formOk}
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

          <details className="group rounded-md border">
            <summary className="flex cursor-pointer list-none items-center justify-between px-3 py-2 text-sm font-medium">
              手动安装
              <span className="text-muted-foreground text-xs group-open:hidden">
                展开
              </span>
              <span className="hidden text-xs text-muted-foreground group-open:inline">
                收起
              </span>
            </summary>
            <div className="space-y-3 border-t px-3 py-3">
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
              <p className="text-muted-foreground text-xs">
                opencode 装好后重启会话即可在{" "}
                <code className="font-mono">skill</code> 工具里看到{" "}
                <code className="font-mono">{params.name || "plocate"}</code>。
              </p>
            </div>
          </details>

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

function FormGrid({ children }: { children: React.ReactNode }) {
  return <div className="grid grid-cols-1 gap-3">{children}</div>
}

function Field({
  label,
  hint,
  error,
  children,
}: {
  label: string
  hint?: string
  error?: string
  children: React.ReactNode
}) {
  return (
    <div className="space-y-1">
      <div className="flex items-baseline justify-between">
        <label className="text-sm font-medium">{label}</label>
        {hint && (
          <span className="text-muted-foreground text-xs">{hint}</span>
        )}
      </div>
      {children}
      {error && <p className="text-destructive text-xs">{error}</p>}
    </div>
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
    <div className="relative">
      <pre className="max-h-60 overflow-auto rounded-md border bg-muted/40 p-3 pr-10 font-mono text-xs leading-relaxed">
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
