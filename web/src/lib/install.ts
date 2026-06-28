export type Agent = "opencode" | "claude" | "codex" | "generic"
export type Target = "global" | "project"

export interface InstallParams {
  name: string
  url: string
  scope: string
  notes: string
  agent: Agent
  target: Target
}

export const DEFAULT_PARAMS: InstallParams = {
  name: "plocate",
  url: "",
  scope: "",
  notes: "",
  agent: "opencode",
  target: "global",
}

const NAME_RE = /^[a-z0-9]+(-[a-z0-9]+)*$/

export function validateSkillName(name: string): boolean {
  return name.length >= 1 && name.length <= 64 && NAME_RE.test(name)
}

export function validateInstanceUrl(url: string): boolean {
  if (url.length > 2048) return false
  const rest = url.startsWith("https://")
    ? url.slice(8)
    : url.startsWith("http://")
      ? url.slice(7)
      : null
  return rest !== null && rest.length > 0
}

function shellQuote(s: string): string {
  return `'${s.replace(/'/g, "'\\''")}'`
}

export function buildInstallUrl(origin: string, p: InstallParams): string {
  const q = new URLSearchParams({
    name: p.name,
    url: p.url,
    agent: p.agent,
    target: p.target,
  })
  if (p.scope) q.set("scope", p.scope)
  if (p.notes) q.set("notes", p.notes)
  return `${origin}/install.sh?${q.toString()}`
}

export function buildOneLiner(origin: string, p: InstallParams): string {
  return `curl -fsSL ${shellQuote(buildInstallUrl(origin, p))} | bash`
}

export function buildMcpAddCommand(p: InstallParams): string {
  switch (p.agent) {
    case "opencode":
      return `opencode mcp add ${p.name} --url ${shellQuote(p.url)}`
    case "claude":
      return `claude mcp add --transport http ${p.name} ${shellQuote(p.url)}`
    case "codex":
      return `codex mcp add ${p.name} --url ${shellQuote(p.url)}`
    case "generic":
      return `# MCP endpoint: ${p.url}`
  }
}

export function buildMcpServersJson(p: InstallParams): string {
  return JSON.stringify(
    {
      mcpServers: {
        [p.name]: {
          type: "http",
          url: p.url,
        },
      },
    },
    null,
    2,
  )
}

export function defaultOrigin(): string {
  if (typeof window === "undefined") return "http://127.0.0.1:8787"
  return window.location.origin
}

export function defaultInstanceUrl(): string {
  return `${defaultOrigin()}/mcp`
}

/**
 * Preview the SKILL.md body by fetching the rendered template from the server.
 * Returns null on error (caller shows a fallback message).
 */
export async function fetchSkillMarkdown(
  origin: string,
  p: InstallParams,
  signal?: AbortSignal,
): Promise<string | null> {
  const q = new URLSearchParams({
    name: p.name,
    url: p.url,
    agent: p.agent,
    target: p.target,
  })
  if (p.scope) q.set("scope", p.scope)
  if (p.notes) q.set("notes", p.notes)
  try {
    const resp = await fetch(`${origin}/install/skill.md?${q.toString()}`, {
      signal,
    })
    if (!resp.ok) return null
    return await resp.text()
  } catch {
    return null
  }
}
