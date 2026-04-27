"use server";

import { z } from "zod";

import { createClient } from "@/lib/supabase/server";

const SignupSchema = z.object({
  email: z.string().trim().toLowerCase().email("Enter a valid email address."),
});

export type WaitlistState =
  | { status: "idle" }
  | { status: "success"; email: string }
  | { status: "error"; message: string };

export async function submitWaitlist(
  _prev: WaitlistState,
  formData: FormData
): Promise<WaitlistState> {
  const parsed = SignupSchema.safeParse({
    email: formData.get("email"),
  });

  if (!parsed.success) {
    return {
      status: "error",
      message: parsed.error.issues[0]?.message ?? "Invalid email.",
    };
  }

  const { email } = parsed.data;
  const supabase = createClient();

  if (!supabase) {
    // Dev fallback: no Supabase env vars yet. Log and pretend it worked so the
    // UI flow is testable end-to-end before Supabase is provisioned.
    console.log("[waitlist:dev]", { email, at: new Date().toISOString() });
    return { status: "success", email };
  }

  const { error } = await supabase
    .from("waitlist")
    .insert({ email, source: "landing" });

  if (error) {
    if (error.code === "23505") {
      // unique_violation — already on the list. Treat as success.
      return { status: "success", email };
    }
    console.error("[waitlist] supabase insert failed", error);
    return { status: "error", message: "Could not save. Try again in a bit." };
  }

  return { status: "success", email };
}
