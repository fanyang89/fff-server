# plocate-web

React + shadcn UI for [plocate-server](../README.md). Filename/path search
with debounced instant results and an MCP configuration sheet.

## Stack

- Vite + React 19 + TypeScript
- Tailwind CSS v4 + shadcn/ui (radix-nova, neutral)
- sonner for toasts, lucide-react for icons

## Develop

The dev server proxies `/api`, `/swagger-ui`, `/openapi.json` to
`http://127.0.0.1:8787` (see `vite.config.ts`). Start the backend first:

```bash
cargo run -- --base-path /srv/files
```

Then in another terminal:

```bash
pnpm install
pnpm dev          # http://localhost:5173
```

## Scripts

| Command       | Description                              |
|---------------|------------------------------------------|
| `pnpm dev`    | Vite dev server with HMR                 |
| `pnpm build`  | Type-check and build into `dist/`        |
| `pnpm lint`   | oxlint                                   |
| `pnpm preview`| Serve the production build locally       |

## Add shadcn components

```bash
pnpm dlx shadcn@latest add <component>
```
