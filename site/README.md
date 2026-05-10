# Faraday Site

Marketing + waitlist site for Faraday. Next.js, deployed separately from the rest of the project.

Not required to use Faraday — the actual signer ([`../`](../)), [browser extension](../extension), and [mobile app](../mobile) work without it.

## Stack

Next.js (App Router) · TypeScript · Tailwind v4 · shadcn/ui · Supabase (waitlist).

## Run

```bash
cd site
pnpm install
pnpm dev
```

Opens at <http://localhost:3000>.

## Database

Schema lives in `supabase/migrations/` and is the source of truth. See [`AGENTS.md`](AGENTS.md) for migration conventions, env vars, and how to push to the remote Supabase project.
