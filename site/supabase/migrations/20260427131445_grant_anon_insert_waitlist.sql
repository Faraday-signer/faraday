-- Anon needs the SQL-level INSERT grant on top of the RLS policy.
-- This Supabase project doesn't grant CRUD to anon by default on public tables,
-- so the previous migration's policy was inert until this grant lands.

grant insert on public.waitlist to anon;
