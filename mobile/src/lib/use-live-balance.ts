import { useEffect, useRef, useState } from "react";

import { address as toAddress } from "@solana/kit";

import { solanaRpcSubscriptions } from "./sol-client";

export type LiveConnectionState =
  | "idle"
  | "connecting"
  | "live"
  | "reconnecting"
  | "failed";

const LOG_PREFIX = "[Faraday][live-balance]";
const BACKOFF_MS = [1_000, 2_000, 4_000, 8_000, 16_000, 30_000] as const;
/**
 * Lower than the extension's 20 — RN's WebSocket subscription path through
 * `@solana/kit` currently hits a structural TypeError on first connect, and
 * retrying just spams the JS console without ever recovering. SWR polling
 * keeps the balance fresh in the meantime; once we either polyfill the
 * missing `@solana/rpc-subscriptions` dep or drop subs entirely on RN, this
 * can come back up.
 */
const MAX_CONSECUTIVE_FAILURES = 2;

export function useLiveBalance(
  pubkey: string | null,
  onChange: () => void
): LiveConnectionState {
  const [state, setState] = useState<LiveConnectionState>("idle");
  const onChangeRef = useRef(onChange);

  useEffect(() => {
    onChangeRef.current = onChange;
  }, [onChange]);

  useEffect(() => {
    if (!pubkey) {
      setState("idle");
      return;
    }

    let cancelled = false;
    let abort = new AbortController();
    let consecutiveFailures = 0;
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

    const addr = (() => {
      try {
        return toAddress(pubkey);
      } catch (error) {
        console.warn(`${LOG_PREFIX} invalid pubkey, bailing out`, error);
        setState("failed");
        return null;
      }
    })();

    if (!addr) return;

    async function runOnce(): Promise<"completed" | "errored"> {
      try {
        setState((prev) => (prev === "idle" ? "connecting" : prev));
        const notifications = await solanaRpcSubscriptions
          .accountNotifications(addr!, { commitment: "confirmed" })
          .subscribe({ abortSignal: abort.signal });

        setState("live");
        consecutiveFailures = 0;
        onChangeRef.current();

        for await (const _notif of notifications) {
          if (cancelled) return "completed";
          onChangeRef.current();
        }
        return "completed";
      } catch (error) {
        if (cancelled) return "completed";
        if (consecutiveFailures === 0) {
          // Log only the first failure per subscription session. Repeated
          // structural errors flood the console without telling us anything new.
          console.warn(`${LOG_PREFIX} subscription error (will fall back to polling)`, error);
        }
        return "errored";
      }
    }

    function scheduleReconnect() {
      if (cancelled) return;
      consecutiveFailures += 1;
      if (consecutiveFailures >= MAX_CONSECUTIVE_FAILURES) {
        setState("failed");
        return;
      }
      setState("reconnecting");
      const delay = BACKOFF_MS[Math.min(consecutiveFailures - 1, BACKOFF_MS.length - 1)];
      reconnectTimer = setTimeout(() => {
        if (cancelled) return;
        abort = new AbortController();
        void loop();
      }, delay);
    }

    async function loop(): Promise<void> {
      const outcome = await runOnce();
      if (cancelled) return;
      if (outcome === "errored") scheduleReconnect();
      else scheduleReconnect();
    }

    void loop();

    return () => {
      cancelled = true;
      abort.abort();
      if (reconnectTimer !== null) {
        clearTimeout(reconnectTimer);
        reconnectTimer = null;
      }
    };
  }, [pubkey]);

  return state;
}
