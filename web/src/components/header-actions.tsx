import { useState } from "react"
import { ArrowUpRight, Code, MoreHorizontal, Plug, Wrench } from "lucide-react"
import { DropdownMenu } from "radix-ui"
import { useTranslation } from "react-i18next"
import { InstallDialog } from "@/components/install-dialog"
import { LanguageSwitcher } from "@/components/language-switcher"
import { MaintenanceDialog } from "@/components/maintenance-dialog"
import { Button } from "@/components/ui/button"
import { withPrefix } from "@/lib/config"

interface HeaderActionsProps {
  instanceName: string
  basePath: string | null
}

export function HeaderActions({
  instanceName,
  basePath,
}: HeaderActionsProps) {
  const { t } = useTranslation()
  const [maintenanceOpen, setMaintenanceOpen] = useState(false)
  const [installOpen, setInstallOpen] = useState(false)

  const moreItems = (
    <>
      <DropdownMenu.Item asChild>
        <a
          href={withPrefix("/swagger-ui")}
          target="_blank"
          rel="noreferrer"
          className="flex cursor-pointer items-center gap-2 rounded-sm px-2 py-1.5 outline-none data-[highlighted]:bg-muted data-[highlighted]:text-foreground"
        >
          <Code className="size-3.5" />
          {t("nav.apiDocs")}
          <ArrowUpRight className="ml-auto size-3 text-muted-foreground" />
        </a>
      </DropdownMenu.Item>
      <DropdownMenu.Item
        onSelect={() => setInstallOpen(true)}
        className="flex cursor-pointer items-center gap-2 rounded-sm px-2 py-1.5 outline-none data-[highlighted]:bg-muted data-[highlighted]:text-foreground"
      >
        <Plug className="size-3.5" />
        {t("install.trigger")}
      </DropdownMenu.Item>
      <DropdownMenu.Item
        onSelect={() => setMaintenanceOpen(true)}
        className="flex cursor-pointer items-center gap-2 rounded-sm px-2 py-1.5 outline-none data-[highlighted]:bg-muted data-[highlighted]:text-foreground"
      >
        <Wrench className="size-3.5" />
        {t("maintenance.trigger")}
      </DropdownMenu.Item>
    </>
  )

  return (
    <div className="flex items-center gap-2">
      <Button asChild variant="ghost" size="sm" className="hidden gap-1.5 sm:inline-flex">
        <a href={withPrefix("/swagger-ui")} target="_blank" rel="noreferrer">
          <Code className="size-3.5" />
          {t("nav.apiDocs")}
          <ArrowUpRight className="size-3 text-muted-foreground" />
        </a>
      </Button>

      <Button
        variant="outline"
        size="sm"
        className="hidden gap-1.5 sm:inline-flex"
        onClick={() => setInstallOpen(true)}
      >
        <Plug className="size-3.5" />
        {t("install.trigger")}
      </Button>

      <Button
        variant="outline"
        size="sm"
        className="hidden gap-1.5 sm:inline-flex"
        onClick={() => setMaintenanceOpen(true)}
      >
        <Wrench className="size-3.5" />
        {t("maintenance.trigger")}
      </Button>

      <DropdownMenu.Root>
        <DropdownMenu.Trigger asChild>
          <Button
            variant="outline"
            size="sm"
            className="sm:hidden"
            aria-label={t("nav.more")}
          >
            <MoreHorizontal className="size-3.5" />
          </Button>
        </DropdownMenu.Trigger>
        <DropdownMenu.Portal>
          <DropdownMenu.Content
            align="end"
            sideOffset={4}
            className="z-50 min-w-40 overflow-hidden rounded-md border bg-popover p-1 text-sm text-popover-foreground shadow-md outline-none data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0"
          >
            {moreItems}
          </DropdownMenu.Content>
        </DropdownMenu.Portal>
      </DropdownMenu.Root>

      <LanguageSwitcher />

      <MaintenanceDialog open={maintenanceOpen} onOpenChange={setMaintenanceOpen} />
      <InstallDialog
        instanceName={instanceName}
        basePath={basePath}
        open={installOpen}
        onOpenChange={setInstallOpen}
      />
    </div>
  )
}
