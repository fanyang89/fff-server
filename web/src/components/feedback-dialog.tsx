import { useEffect, useState } from "react"
import { MessageCircle, Send } from "lucide-react"
import { useTranslation } from "react-i18next"
import { fetchFeedback } from "@/api"
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
  const { t } = useTranslation()
  const [open, setOpen] = useState(false)
  const [message, setMessage] = useState("")
  const [email, setEmail] = useState<string | null | undefined>(undefined)

  useEffect(() => {
    const ctrl = new AbortController()
    fetchFeedback(ctrl.signal).then((c) => setEmail(c.email)).catch(() => setEmail(null))
    return () => ctrl.abort()
  }, [])

  if (!email) return null

  const subject = encodeURIComponent(t("feedback.subject"))
  const body = encodeURIComponent(message)
  const mailto = `mailto:${email}?subject=${subject}${message ? `&body=${body}` : ""}`

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <button className="inline-flex items-center gap-1 transition-colors hover:text-foreground">
          <MessageCircle className="size-3" />
          {t("feedback.trigger")}
        </button>
      </DialogTrigger>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>{t("feedback.title")}</DialogTitle>
          <DialogDescription>{t("feedback.description")}</DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="space-y-1.5">
            <span className="text-muted-foreground text-xs">{t("feedback.recipientLabel")}</span>
            <code className="block select-all rounded-md border bg-muted/40 px-3 py-2 font-mono text-sm">
              {email}
            </code>
          </div>

          <div className="space-y-1.5">
            <span className="text-muted-foreground text-xs">{t("feedback.bodyLabel")}</span>
            <Textarea
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              placeholder={t("feedback.bodyPlaceholder")}
              rows={5}
            />
          </div>

          <Button asChild className="w-full gap-2">
            <a href={mailto} onClick={() => setOpen(false)}>
              <Send className="size-4" />
              {t("feedback.send")}
            </a>
          </Button>

          <p className="text-muted-foreground text-xs">{t("feedback.mailtoHint")}</p>
        </div>
      </DialogContent>
    </Dialog>
  )
}
