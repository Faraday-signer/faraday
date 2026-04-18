import { useEffect, useRef, useState } from "react";

import { address as toAddress } from "@solana/kit";

import { solanaRpcSubscriptions } from "./sol-client";

export type LiveConnectionState =
  | "idle" // no pubkey, nothing to subscribe to
  | "connecting" // first attempt in flight
  | "live" // connected, subscription active, pushes flowing
  | "reconnecting" // disconnected, backoff in progress
  | "failed"; // gave up after max retries

const LOG_PREFIX = "[Faraday][live-balance]";

const BACKOFF_MS = [1_000, 2_000, 4_000, 8_000, 16_000, 30_000] as const;
/** Give up as "failed" only after this many consecutive failures. */
const MAX_CONSECUTIVE_FAILURES = 20;

/**
 * Subscribe to on-chain balance changes for `pubkey`. Calls `onChange`
 * whenever the server pushes an account update at the `confirmed`
 * commitment level.
 *
 * Pairs with SWR polling in `useWallet` — the subscription is the fast
 * path, SWR is the backstop. When this hook reconnects after an outage,
 * it fires `onChange` once eagerly so the SWR cache gets re-validated
 * against ground truth (we might have missed notifications while
 * disconnected).
 *
 * Subscriptions live in the side-panel document context, not in the
 * service worker, so MV3 worker eviction doesn't kill them.
 */
export function useLiveBalance(
  pubkey: string | null,
  onChange: () => void
): LiveConnectionState {
  const [state, setState] = useState<LiveConnectionState>("idle");
  const onChangeRef = useRef(onChange);

  // Keep the latest onChange without restarting the subscription on every
  // render. The subscription lifecycle is keyed to pubkey alone.
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

        // Fire once on (re)connect so SWR revalidates against ground
        // truth — we may have missed updates while disconnected.
        onChangeRef.current();

        for await (const _notif of notifications) {
          if (cancelled) return "completed";
          onChangeRef.current();
        }
        return "completed";
      } catch (error) {
        if (cancelled) return "completed";
        console.warn(`${LOG_PREFIX} subscription error`, error);
        return "errored";
      }
    }

    function scheduleReconnect() {
      if (cancelled) return;
      consecutiveFailures += 1;
      if (consecutiveFailures >= MAX_CONSECUTIVE_FAILURES) {
        console.error(
          `${LOG_PREFIX} giving up after ${consecutiveFailures} failures`
        );
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
      if (outcome === "errored") {
        scheduleReconnect();
      } else {
        // completed normally (server closed); treat as reconnectable
        scheduleReconnect();
      }
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
