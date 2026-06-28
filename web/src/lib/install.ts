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
  const project = p.target === "project"
  switch (p.agent) {
    case "opencode":
      if (project) {
        // opencode's `mcp add` always writes the global config; project scope
        // needs a manual edit of .opencode/opencode.jsonc (note the key is
        // "mcp", not the Cursor-style "mcpServers").
        return [
          "# opencode 项目级配置需手动编辑 .opencode/opencode.jsonc",
          '# 注意 opencode 用 "mcp" 键（不是 "mcpServers"）：',
          '"mcp": {',
          `  ${JSON.stringify(p.name)}: { "type": "http", "url": ${JSON.stringify(p.url)} }`,
          "}",
        ].join("\n")
      }
      return `opencode mcp add ${p.name} --url ${shellQuote(p.url)}`
    case "claude":
      // claude mcp add supports -s user|local|project natively.
      // global → user scope; project → local scope (default, no flag needed).
      return project
        ? `claude mcp add --transport http ${p.name} ${shellQuote(p.url)}`
        : `claude mcp add -s user --transport http ${p.name} ${shellQuote(p.url)}`
    case "codex":
      if (project) {
        // codex's `mcp add` always writes ~/.codex/config.toml; project scope
        // needs a manual edit of .codex/config.toml (TOML format).
        return [
          "# codex 项目级配置需手动编辑 .codex/config.toml",
          "[mcp_servers." + p.name + "]",
          `url = ${JSON.stringify(p.url)}`,
        ].join("\n")
      }
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
