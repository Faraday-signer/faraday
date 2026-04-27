-- Faraday waitlist
--
-- Public landing-page form writes here. Anon role may INSERT only; the list is
-- private (no anon SELECT/UPDATE/DELETE). Admins read via the service-role key
-- which bypasses RLS.

create extension if not exists "pgcrypto";

create table if not exists public.waitlist (
  id uuid primary key default gen_random_uuid(),
  email text not null,
  source text,
  user_agent text,
  created_at timestamptz not null default now(),
  constraint waitlist_email_unique unique (email)
);

create index if not exists waitlist_created_at_idx
  on public.waitlist (created_at desc);

alter table public.waitlist enable row level security;

drop policy if exists "anon_insert_waitlist" on public.waitlist;
create policy "anon_insert_waitlist"
  on public.waitlist
  for insert
  to anon
  with check (true);
