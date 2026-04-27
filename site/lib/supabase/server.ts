import { createClient as createSupabaseClient } from "@supabase/supabase-js";

/**
 * Server-only Supabase client for the public waitlist.
 *
 * Uses the anon key + RLS for safety — RLS policies in lib/db/schema.sql
 * restrict anon to INSERT only. Returns null when env vars are missing so the
 * Server Action can fall back to a dev-log path.
 */
export function createClient() {
  const url = process.env.SUPABASE_URL;
  const anonKey = process.env.SUPABASE_ANON_KEY;
  if (!url || !anonKey) return null;

  return createSupabaseClient(url, anonKey, {
    auth: { persistSession: false },
  });
}
