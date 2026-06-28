export type Agent = "opencode" | "claude" | "codex" | "generic"
export type Target = "global" | "project"

export interface InstallParams {
  name: string
  url: string
  agent: Agent
  target: Target
}

function shellQuote(s: string): string {
  return `'${s.replace(/'/g, "'\\''")}'`
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
