"use client";

import { useActionState } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { submitWaitlist, type WaitlistState } from "@/app/actions/waitlist";

const initialState: WaitlistState = { status: "idle" };

export function WaitlistForm() {
  const [state, action, pending] = useActionState(submitWaitlist, initialState);

  if (state.status === "success") {
    return (
      <div className="flex flex-col gap-1">
        <p className="font-display text-sm uppercase tracking-wider text-foreground">
          ✓ You're on the list.
        </p>
        <p className="text-sm text-muted-foreground">
          We'll reach out at <span className="font-mono">{state.email}</span> when devices ship.
        </p>
      </div>
    );
  }

  return (
    <form action={action} className="flex flex-col gap-2">
      <div className="flex flex-col gap-2 sm:flex-row">
        <Input
          type="email"
          name="email"
          required
          autoComplete="email"
          placeholder="you@domain.com"
          aria-label="Email address"
          className="sm:flex-1"
        />
        <Button type="submit" disabled={pending} className="sm:w-auto">
          {pending ? "Joining…" : "Join the list →"}
        </Button>
      </div>
      {state.status === "error" ? (
        <p className="text-xs text-red-700" role="alert">
          {state.message}
        </p>
      ) : null}
    </form>
  );
}
