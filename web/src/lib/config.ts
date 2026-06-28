// Public path prefix the SPA is served under. Injected by the Rust server
// into index.html as <script>window.__BASE_PATH__="/search"</script> when
// --public-base-url is configured. Empty string for root mount.
//
// All in-app URL construction (fetch, MCP install links, Swagger link) must
// go through basePath() so the SPA keeps working regardless of where it is
// mounted. Read once at module load — the value does not change after boot.
declare global {
  interface Window {
    __BASE_PATH__?: string
  }
}

const RAW = typeof window !== "undefined" ? window.__BASE_PATH__ ?? "" : ""

// Defensive normalization: strip trailing slash, keep leading slash or "".
export const BASE_PATH: string = RAW.endsWith("/") && RAW.length > 1
  ? RAW.slice(0, -1)
  : RAW

/** Prefix a root-absolute app path (`/api/...`, `/swagger-ui`, `/mcp`) with BASE_PATH. */
export function withPrefix(path: string): string {
  if (!path.startsWith("/")) path = "/" + path
  return BASE_PATH + path
}
