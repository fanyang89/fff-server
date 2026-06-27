import { useState } from "react"
import { Check, Copy, ExternalLink, Plug, Wrench } from "lucide-react"
import { toast } from "sonner"
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

const SERVER_URL = "http://127.0.0.1:8787/mcp"

// VS Code / mcp.json style — widely adopted by Cursor, Cline, etc.
const MCP_CONFIG = `{
  "mcpServers": {
    "plocate-server": {
      "url": "${SERVER_URL}",
      "type": "http"
    }
  }
}`

const TOOLS = [
  {
    name: "search_files",
    desc: "按子串或通配符搜索文件路径",
  },
  {
    name: "glob",
    desc: "按 glob 模式搜索，如 *.rs、**/2024/*.log",
  },
]

export function McpDialog() {
  const [open, setOpen] = useState(false)
  const [copied, setCopied] = useState(false)

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(MCP_CONFIG)
      setCopied(true)
      toast.success("已复制配置")
      setTimeout(() => setCopied(false), 1200)
    } catch {
      toast.error("复制失败")
    }
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button variant="outline" size="sm" className="gap-2">
          <Plug className="size-4" />
          配置 MCP
        </Button>
      </DialogTrigger>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>配置 MCP</DialogTitle>
          <DialogDescription>
            让 AI 客户端通过 Model Context Protocol 直接调用本服务的文件搜索。
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-5">
          <section className="space-y-2">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium">客户端配置</span>
              <Button
                variant="ghost"
                size="sm"
                className="h-7 gap-1.5 px-2"
                onClick={copy}
              >
                {copied ? (
                  <Check className="size-3.5" />
                ) : (
                  <Copy className="size-3.5" />
                )}
                复制
              </Button>
            </div>
            <pre className="overflow-x-auto rounded-md border bg-muted/40 p-3 font-mono text-xs leading-relaxed">
              {MCP_CONFIG}
            </pre>
            <p className="text-muted-foreground text-xs">
              端点：<code className="font-mono">{SERVER_URL}</code>
            </p>
          </section>

          <section className="space-y-2">
            <div className="flex items-center gap-2 text-sm font-medium">
              <Wrench className="size-4" />
              可用工具
            </div>
            <ul className="space-y-2">
              {TOOLS.map((t) => (
                <li
                  key={t.name}
                  className="flex flex-col gap-0.5 rounded-md border p-2.5"
                >
                  <code className="font-mono text-xs font-medium">
                    {t.name}
                  </code>
                  <span className="text-muted-foreground text-xs">
                    {t.desc}
                  </span>
                </li>
              ))}
            </ul>
            <div className="flex items-center gap-1.5 pt-1">
              <Badge variant="secondary" className="font-normal">
                Streamable HTTP
              </Badge>
              <Badge variant="secondary" className="font-normal">
                Stateless
              </Badge>
            </div>
          </section>
        </div>

        <div className="flex justify-end">
          <Button asChild variant="link" size="sm" className="h-auto gap-1.5 px-0">
            <a
              href="https://modelcontextprotocol.io"
              target="_blank"
              rel="noreferrer"
            >
              <ExternalLink className="size-3.5" />
              查看 MCP 文档
            </a>
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
