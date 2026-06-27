import { useState } from "react"
import { HelpCircle } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip"

const RULES = [
  {
    title: "多关键字模糊搜索",
    desc: "空格分隔的多个关键字需同时命中，顺序不限，结果按相关性排序。",
  },
  {
    title: "含通配符时按 glob 解析",
    desc: "输入中出现 * ? [ ] 之一即作为 glob 模式，默认在路径任意位置匹配；以 / 开头可锚定到路径根。",
  },
]

const EXAMPLES = [
  { input: "Cargo.toml", desc: "路径中包含此子串" },
  { input: "*.rs", desc: "所有 Rust 源文件" },
  { input: "rust*json", desc: "路径任意位置含 rust…json，如 .rustc_info.json" },
  { input: "**/2024/*.log", desc: "任意层级 2024 目录下的日志" },
  { input: "/etc/*.conf", desc: "仅 /etc 目录下的 .conf 文件" },
  { input: "config json", desc: "同时含 config 和 json（顺序不限）" },
]

export function SyntaxHelpTrigger() {
  const [open, setOpen] = useState(false)

  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger asChild>
          <button
            type="button"
            aria-label="搜索语法"
            onClick={() => setOpen(true)}
            className="flex size-5 items-center justify-center rounded-full text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
          >
            <HelpCircle className="size-3.5" />
          </button>
        </TooltipTrigger>
        <TooltipContent>搜索语法</TooltipContent>
      </Tooltip>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>搜索语法</DialogTitle>
            <DialogDescription>
              底层由 plocate 提供索引检索，以下规则决定输入如何被解析。
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-5">
            <section className="space-y-2">
              <span className="text-sm font-medium">匹配规则</span>
              <ul className="space-y-2">
                {RULES.map((r) => (
                  <li
                    key={r.title}
                    className="flex flex-col gap-0.5 rounded-md border p-2.5"
                  >
                    <span className="text-xs font-medium">{r.title}</span>
                    <span className="text-muted-foreground text-xs">
                      {r.desc}
                    </span>
                  </li>
                ))}
              </ul>
            </section>

            <section className="space-y-2">
              <span className="text-sm font-medium">示例</span>
              <ul className="divide-y rounded-md border">
                {EXAMPLES.map((ex) => (
                  <li
                    key={ex.input}
                    className="flex items-center gap-3 px-3 py-2"
                  >
                    <code className="shrink-0 font-mono text-xs font-medium">
                      {ex.input}
                    </code>
                    <span className="text-muted-foreground text-xs">
                      {ex.desc}
                    </span>
                  </li>
                ))}
              </ul>
            </section>

            <section className="space-y-2">
              <span className="text-sm font-medium">当前界面行为</span>
              <div className="flex flex-wrap items-center gap-1.5">
                <Badge variant="secondary" className="font-normal">
                  大小写：不敏感
                </Badge>
                <Badge variant="secondary" className="font-normal">
                  范围：全路径
                </Badge>
                <Badge variant="secondary" className="font-normal">
                  上限：100 条
                </Badge>
              </div>
              <p className="text-muted-foreground text-xs">
                更多参数（limit / offset / case）可通过{" "}
                <code className="font-mono">/api/fuzzy</code>{" "}
                查询参数覆盖，详见 API 文档。
              </p>
            </section>
          </div>
        </DialogContent>
      </Dialog>
    </TooltipProvider>
  )
}
