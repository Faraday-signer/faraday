<!-- BEGIN:nextjs-agent-rules -->
# This is NOT the Next.js you know

This version has breaking changes — APIs, conventions, and file structure may all differ from your training data. Read the relevant guide in `node_modules/next/dist/docs/` before writing any code. Heed deprecation notices.
<!-- END:nextjs-agent-rules -->

# Database — Supabase + migrations

## Golden rule

**Never change the remote database directly.** No clicks in the Supabase Studio UI to add columns. No ad-hoc SQL pasted into the SQL editor. Every schema change goes through a migration file in `supabase/migrations/`, which is committed to git, reviewed in a PR, and deployed via `supabase db push`.

## Layout

- `supabase/migrations/<timestamp>_<name>.sql` — schema changes (the source of truth)
- `supabase/config.toml` — local CLI config (committed)
- `supabase/.gitignore` — CLI-managed, leave it alone

There is no `lib/db/schema.sql`. If you see one, it's stale — delete it. Migrations are authoritative.

## Authoring a migration

1. Generate a timestamped file:
   ```
   pnpm exec supabase migration new <snake_case_description>
   ```
   This writes `supabase/migrations/<YYYYMMDDHHMMSS>_<description>.sql`. Use the CLI for the timestamp — don't hand-roll the filename, ordering matters.

2. Write SQL in that file. Conventions:
   - **Idempotent DDL.** `create table if not exists`, `create index if not exists`, `alter table … add column if not exists`, `drop policy if exists` before `create policy`. A migration that's safe to re-run is safe to roll forward.
   - **RLS in the same migration as the table.** Every `public.<table>` enables RLS in the file that creates it, with explicit policies. Default-deny is the floor; loosen explicitly per role.
   - **Least-privilege for anon.** A public form table gets `INSERT to anon` only. Never `to authenticated, anon` unless you mean both. Never grant anon `SELECT` on data that should be private.
   - **One concern per migration.** Adding a column + a new table = two migrations. Easier to review, easier to revert.
   - **Never edit a pushed migration.** Once `supabase db push` has been run against any environment, that file is frozen. To change the schema again, write a new migration. Editing history breaks `db push` (it tracks applied migrations by timestamp).

3. Test locally before pushing (optional, only if you've started the local stack with `supabase start`):
   ```
   pnpm exec supabase db reset
   ```
   This drops and rebuilds the local DB from migrations, so it catches ordering and idempotency bugs.

## Pushing to the remote project

The Faraday project is `cxjjokfggtenlzfnsukq`.

One-time setup per machine:

1. Generate a Personal Access Token at <https://supabase.com/dashboard/account/tokens>.
2. Authenticate:
   ```
   pnpm exec supabase login --token <PAT>
   ```
   (or `supabase login` for the browser flow). Token is stored in `~/.supabase/`.
3. Get the database password from the Supabase dashboard → **Settings → Database → Connection string** (or **Reset database password** if you don't have it). This is *not* the service-role key.
4. Link the local repo to the remote project:
   ```
   pnpm exec supabase link --project-ref cxjjokfggtenlzfnsukq -p <db-password>
   ```
   Stores the project ref in `supabase/.temp/` (gitignored).

Each push:

```
pnpm exec supabase db push
```

Applies any unapplied migrations to the remote, in timestamp order. Run `supabase migration list` first if you want to see what's pending.

## Env vars

Set in `.env.local` (gitignored). Mirrored without values in `.env.example`.

| Var | Used by | Notes |
| --- | --- | --- |
| `SUPABASE_URL` | Server Action (waitlist) | Public URL, but kept server-side for clean boundary |
| `SUPABASE_ANON_KEY` | Server Action (waitlist) | RLS-restricted; safe in principle but server-only here |
| `SUPABASE_SERVICE_ROLE_KEY` | Future admin tooling only | **Bypasses RLS.** Never read from app code that handles user input |
| `SUPABASE_ACCESS_TOKEN` | CLI (optional) | Personal Access Token; alternative to `supabase login` |

The waitlist Server Action (`app/actions/waitlist.ts`) uses the **anon key only** and relies on RLS for safety. Never refactor it to use the service-role key — that would defeat the policy.

## Code conventions

- DB clients live in `lib/supabase/`. Currently only `server.ts` (waitlist insert). Add a `service.ts` if/when admin tooling needs the service-role key — never mix it into `server.ts`.
- Always pass `auth: { persistSession: false }` to clients used in stateless server contexts. The default tries to write a refresh-token cookie, which is wrong for a Server Action.
