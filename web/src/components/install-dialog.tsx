import { useEffect, useMemo, useState } from "react"
import { Check, Copy, ExternalLink } from "lucide-react"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
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
  open: boolean
  onOpenChange: (v: boolean) => void
}

const AGENT_VALUES: Agent[] = ["opencode", "claude", "codex", "generic"]
const TARGET_VALUES: Target[] = ["global", "project"]
const TAB_VALUES: { value: Tab; labelKey: string }[] = [
  { value: "mcp", labelKey: "install.tabMcp" },
  { value: "json", labelKey: "install.tabJson" },
]

export function InstallDialog({
  instanceName,
  basePath,
  open,
  onOpenChange,
}: InstallDialogProps) {
  const { t } = useTranslation()
  const [agent, setAgent] = useState<Agent>("opencode")
  const [target, setTarget] = useState<Target>("global")
  const [tab, setTab] = useState<Tab>("mcp")
  const [copied, setCopied] = useState(false)
  const [origin, setOrigin] = useState("")

  const ready = basePath !== null
  const url = origin ? `${origin}${withPrefix("/mcp")}` : ""
  const secure = origin.startsWith("https://")

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
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-xl sm:max-w-xl max-h-[calc(100dvh-2rem)] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{t("install.title")}</DialogTitle>
          <DialogDescription>{t("install.description")}</DialogDescription>
        </DialogHeader>

        <div className="min-w-0 space-y-4">
          <section className="space-y-2">
            <span className="text-sm font-medium">{t("install.instanceInfo")}</span>
            <div className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1.5 rounded-md border bg-muted/30 p-3 text-xs">
              <span className="text-muted-foreground">{t("install.name")}</span>
              <code className="font-mono">{instanceName || "plocate"}</code>
              <span className="text-muted-foreground">{t("install.endpoint")}</span>
              <code className="min-w-0 break-all font-mono">{url || t("install.placeholderEndpoint")}</code>
              <span className="text-muted-foreground">{t("install.scope")}</span>
              {ready ? (
                <code className="min-w-0 break-all font-mono">{basePath}</code>
              ) : (
                <Skeleton className="h-4 w-40" />
              )}
            </div>
          </section>

          <section className="space-y-2">
            <span className="text-sm font-medium">{t("install.installTarget")}</span>
            <div className="grid grid-cols-2 gap-3">
              <RadioGroup
                label={t("install.agentLabel")}
                value={agent}
                options={AGENT_VALUES.map((a) => ({
                  value: a,
                  label: t(`install.agents.${a}.label`),
                  hint: t(`install.agents.${a}.hint`),
                }))}
                onChange={(v) => setAgent(v as Agent)}
              />
              <RadioGroup
                label={t("install.targetLabel")}
                value={target}
                options={TARGET_VALUES.map((v) => ({
                  value: v,
                  label: t(`install.targets.${v}`),
                }))}
                onChange={(v) => setTarget(v as Target)}
              />
            </div>
          </section>

          <section className="space-y-2">
            <div className="flex gap-1 rounded-md bg-muted/40 p-1">
              {TAB_VALUES.map((tTab) => (
                <button
                  key={tTab.value}
                  type="button"
                  onClick={() => setTab(tTab.value)}
                  className={cn(
                    "flex-1 rounded-[inherit] px-3 py-1.5 text-sm font-medium transition-colors",
                    tab === tTab.value
                      ? "bg-background text-foreground shadow-sm"
                      : "text-muted-foreground hover:text-foreground",
                  )}
                >
                  {t(tTab.labelKey)}
                </button>
              ))}
            </div>
            <SnippetBlock
              value={current}
              copied={copied}
              onCopy={() => copy(current)}
              copyable={secure}
            />
            <p className="text-muted-foreground text-xs">{t("install.copyHint")}</p>
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
                {t("install.mcpDocs")}
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
  copyable = true,
}: {
  value: string
  copied: boolean
  onCopy: () => void
  copyable?: boolean
}) {
  const { t } = useTranslation()
  return (
    <div className="relative min-w-0">
      <pre className="max-h-60 min-w-0 overflow-auto whitespace-pre rounded-md border bg-muted/40 p-3 font-mono text-xs leading-relaxed">
        {value || t("install.generating")}
      </pre>
      {copyable && (
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
      )}
    </div>
  )
}
