import { useState } from "react"
import { MessageCircle, Send } from "lucide-react"
import { FEEDBACK_EMAIL } from "@/config"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog"
import { Textarea } from "@/components/ui/textarea"

export function FeedbackDialog() {
  const [open, setOpen] = useState(false)
  const [message, setMessage] = useState("")

  if (!FEEDBACK_EMAIL) return null

  const subject = encodeURIComponent("[plocate-web] 反馈")
  const body = encodeURIComponent(message)
  const mailto = `mailto:${FEEDBACK_EMAIL}?subject=${subject}${message ? `&body=${body}` : ""}`

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <button className="inline-flex items-center gap-1 transition-colors hover:text-foreground">
          <MessageCircle className="size-3" />
          反馈问题
        </button>
      </DialogTrigger>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>反馈问题</DialogTitle>
          <DialogDescription>
            欢迎报告 bug、提出建议或反馈体验。
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="space-y-1.5">
            <span className="text-muted-foreground text-xs">收件地址</span>
            <code className="block select-all rounded-md border bg-muted/40 px-3 py-2 font-mono text-sm">
              {FEEDBACK_EMAIL}
            </code>
          </div>

          <div className="space-y-1.5">
            <span className="text-muted-foreground text-xs">
              内容（可选，会带入邮件正文）
            </span>
            <Textarea
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              placeholder="描述你遇到的问题或想法…"
              rows={5}
            />
          </div>

          <Button asChild className="w-full gap-2">
            <a href={mailto} onClick={() => setOpen(false)}>
              <Send className="size-4" />
              发送邮件
            </a>
          </Button>

          <p className="text-muted-foreground text-xs">
            点击后若未弹出邮件应用，可手动复制上方地址至网页邮箱。
          </p>
        </div>
      </DialogContent>
    </Dialog>
  )
}
