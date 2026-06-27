import { useState } from "react"
import { Check, Copy, ExternalLink, Plug } from "lucide-react"
import { toast } from "sonner"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet"

// TODO: replace with real MCP schema once the MCP agent lands.
const MCP_CONFIG = `{
  "mcpServers": {
    "fff-server": {
      "url": "http://127.0.0.1:8787"
    }
  }
}`

export function McpSheet() {
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
    <Sheet open={open} onOpenChange={setOpen}>
      <SheetTrigger asChild>
        <Button variant="outline" size="sm" className="gap-2">
          <Plug className="size-4" />
          配置 MCP
        </Button>
      </SheetTrigger>
      <SheetContent className="w-full sm:max-w-md">
        <SheetHeader>
          <div className="flex items-center gap-2">
            <SheetTitle>配置 MCP</SheetTitle>
            <Badge variant="secondary" className="font-normal">
              Coming soon
            </Badge>
          </div>
          <SheetDescription>
            让 AI agent 直接调用本服务的文件搜索能力。
          </SheetDescription>
        </SheetHeader>

        <div className="mt-6 space-y-3">
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium">配置示例</span>
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

          <Button
            variant="link"
            size="sm"
            className="h-auto gap-1.5 px-0"
            disabled
          >
            <ExternalLink className="size-3.5" />
            查看 MCP 文档
          </Button>
        </div>
      </SheetContent>
    </Sheet>
  )
}
