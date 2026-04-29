//! Pre-flight validation for the sidepanel Send flow.
//!
//! Two layers:
//!   1. `validateAmount` / `validateRecipientFormat` — pure sync checks
//!      that map a user-input string to a structured verdict. Used to
//!      render inline error/warning text under each field as the user
//!      types and to gate the Review button.
//!   2. `useRecipientCheck` — async hook that adds an RPC layer on top
//!      of the format check: detects whether the recipient is a program
//!      (would lock funds), whether it's a fresh account that needs a
//!      rent-exempt minimum, and whether it's the user's own address.
//!
//! The verdict shape is the same across both:
//!   - `kind: "ok"` — proceed
//!   - `kind: "warn"` — allow but surface a yellow inline note
//!   - `kind: "error"` — block, show in red, disable Review
//!   - `kind: "loading"` — async check in flight (recipient only)
//!
//! All amount math is done in atoms (bigint) — never via Number — to
//! avoid float-precision losses for tokens with high decimal counts.

import useSWR from "swr";
import {
  address as toAddress,
  type Address,
} from "@solana/kit";

import { isValidSolanaAddress } from "./solana";
import { solanaRpc } from "./sol-client";

/** One signature's base fee. SOL only. */
export const FEE_RESERVE_LAMPORTS = 5000n;

/**
 * Rent-exempt minimum for a fresh system-owned account (size 0). Sending
 * less than this to a non-existent address means the account never lands —
 * the lamports are returned to the sender, but with a wasted fee.
 */
export const RENT_EXEMPT_MIN_LAMPORTS = 890_880n;

export type AmountCheck =
  | { kind: "ok"; raw: bigint }
  | { kind: "warn"; raw: bigint; message: string }
  | { kind: "error"; message: string };

interface ValidateAmountInput {
  /** Raw user input (the contents of the amount field). */
  amountStr: string;
  /** Spendable balance in atoms (lamports for SOL). Null while loading. */
  balanceRaw: bigint | null;
  /** Mint decimals — 9 for SOL. */
  decimals: number;
  /** Symbol for messages — "SOL", "USDC", etc. */
  symbol: string;
  /**
   * Lamports to leave in reserve for the network fee. Pass `FEE_RESERVE_LAMPORTS`
   * for SOL transfers; pass 0n for SPL (the fee is paid in SOL out of a
   * separate balance, so the SPL amount can equal the SPL balance).
   */
  feeReserve: bigint;
}

/**
 * Pure sync validation for the amount field. Returns a verdict ready to
 * render under the input.
 */
export function validateAmount(input: ValidateAmountInput): AmountCheck {
  const { amountStr, balanceRaw, decimals, symbol, feeReserve } = input;
  const trimmed = amountStr.trim();

  if (trimmed.length === 0) {
    return { kind: "error", message: "Enter an amount." };
  }

  if (!/^\d+(\.\d+)?$/.test(trimmed)) {
    return { kind: "error", message: "Amount must be a positive number." };
  }

  const [whole, frac = ""] = trimmed.split(".");
  if (frac.length > decimals) {
    return {
      kind: "error",
      message:
        decimals === 0
          ? `${symbol} has no decimals.`
          : `Max ${decimals} decimal${decimals === 1 ? "" : "s"} for ${symbol}.`,
    };
  }

  const factor = 10n ** BigInt(decimals);
  const padded = decimals > 0
    ? (frac + "0".repeat(decimals)).slice(0, decimals)
    : "";
  const raw = BigInt(whole) * factor + (decimals > 0 ? BigInt(padded) : 0n);

  if (raw <= 0n) {
    return { kind: "error", message: "Amount must be greater than zero." };
  }

  // Balance is still loading — accept the parse but let the upstream UI
  // disable Review until the balance lands.
  if (balanceRaw === null) {
    return { kind: "ok", raw };
  }

  if (raw > balanceRaw) {
    return {
      kind: "error",
      message: `Exceeds balance — max ${formatRaw(balanceRaw, decimals)} ${symbol}`,
    };
  }

  if (feeReserve > 0n && raw + feeReserve > balanceRaw) {
    return {
      kind: "warn",
      raw,
      message: `Leave ~${formatRaw(feeReserve, decimals)} ${symbol} for the fee.`,
    };
  }

  return { kind: "ok", raw };
}

/** Pure sync recipient format check. */
export function validateRecipientFormat(
  recipient: string
): { kind: "ok" } | { kind: "error"; message: string } {
  const trimmed = recipient.trim();
  if (trimmed.length === 0) return { kind: "error", message: "" };
  if (!isValidSolanaAddress(trimmed)) {
    return { kind: "error", message: "Not a valid Solana address." };
  }
  return { kind: "ok" };
}

export type RecipientCheck =
  | { kind: "loading" }
  | { kind: "ok"; note?: string }
  | { kind: "warn"; message: string }
  | { kind: "error"; message: string };

/**
 * Async recipient check — fetches `getAccountInfo` to flag programs (would
 * lock funds), self-sends, fresh accounts (informational), and rent-exempt
 * minimums. Returns `{ kind: "ok" }` for empty / format-invalid input so
 * the format check (rendered separately) doesn't get duplicated.
 */
export function useRecipientCheck(
  recipient: string,
  ownPubkey: string | null,
  intendedRaw: bigint | null
): RecipientCheck {
  const trimmed = recipient.trim();
  const formatValid =
    trimmed.length > 0 && isValidSolanaAddress(trimmed);

  // We always call useSWR so React's hook order stays stable — but we pass
  // null as the key to skip the actual fetch when the address isn't ready.
  const { data, error, isLoading } = useSWR(
    formatValid ? ["recipient-info", trimmed] : null,
    async ([, addr]: readonly [string, string]) => {
      const info = await solanaRpc
        .getAccountInfo(toAddress(addr) as Address, { encoding: "base64" })
        .send();
      return info.value; // null when the account doesn't exist
    },
    {
      revalidateOnFocus: false,
      dedupingInterval: 10_000,
      shouldRetryOnError: false,
      keepPreviousData: false,
    }
  );

  // Empty / format-invalid: stay silent. The format helper text is rendered
  // separately by the screen so we don't double up.
  if (trimmed.length === 0 || !formatValid) {
    return { kind: "ok" };
  }

  // Self-send. Cheap to check — do this before the network result lands so
  // the user gets immediate feedback.
  if (ownPubkey && trimmed === ownPubkey) {
    return { kind: "warn", message: "This is your own address." };
  }

  if (isLoading) return { kind: "loading" };

  // RPC errored — don't block the user. The build-time path will surface
  // any real failure, and we'd rather not turn a flaky RPC into a UX wall.
  if (error) return { kind: "ok" };

  const accountInfo = data;

  // Programs would lock the funds. Hard block.
  if (accountInfo && accountInfo.executable) {
    return {
      kind: "error",
      message: "Recipient is a program — funds would be locked.",
    };
  }

  // Fresh account — exists nowhere on chain yet. The amount must reach the
  // rent-exempt floor or the lamports just bounce. Show as a warning rather
  // than a hard block: the bounce is recoverable, the user might know what
  // they're doing.
  if (accountInfo === null) {
    if (
      intendedRaw !== null &&
      intendedRaw > 0n &&
      intendedRaw < RENT_EXEMPT_MIN_LAMPORTS
    ) {
      return {
        kind: "warn",
        message: `Fresh accounts need ≥ ${formatRaw(RENT_EXEMPT_MIN_LAMPORTS, 9)} SOL to stay alive.`,
      };
    }
    return { kind: "ok", note: "Fresh address — first transaction here." };
  }

  return { kind: "ok" };
}

/** Format a raw atom amount for display. Trims trailing zeros after the decimal. */
function formatRaw(raw: bigint, decimals: number): string {
  if (decimals === 0) return raw.toString();
  const factor = 10n ** BigInt(decimals);
  const whole = raw / factor;
  const frac = raw % factor;
  if (frac === 0n) return whole.toString();
  const fracStr = frac.toString().padStart(decimals, "0").replace(/0+$/, "");
  return `${whole.toString()}.${fracStr}`;
}
