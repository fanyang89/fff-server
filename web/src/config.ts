// Build-time configuration (Vite env vars, prefixed with VITE_).
// Set at build time, e.g.  VITE_FEEDBACK_EMAIL=foo@bar.com pnpm build

export const FEEDBACK_EMAIL = import.meta.env.VITE_FEEDBACK_EMAIL?.trim() || undefined
