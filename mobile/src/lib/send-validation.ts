import useSWR from "swr";
import { address as toAddress, type Address } from "@solana/kit";

import { isValidSolanaAddress } from "./solana";
import { solanaRpc } from "./sol-client";

export const FEE_RESERVE_LAMPORTS = 10_000n;
export const RENT_EXEMPT_MIN_LAMPORTS = 890_880n;

export type AmountCheck =
  | { kind: "ok"; raw: bigint }
  | { kind: "warn"; raw: bigint; message: string }
  | { kind: "error"; message: string };

interface ValidateAmountInput {
  amountStr: string;
  balanceRaw: bigint | null;
  decimals: number;
  symbol: string;
  feeReserve: bigint;
}

export function validateAmount(input: ValidateAmountInput): AmountCheck {
  const { amountStr, balanceRaw, decimals, symbol, feeReserve } = input;
  const trimmed = amountStr.trim();

  if (trimmed.length === 0) return { kind: "error", message: "Enter an amount." };
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
          : `Max ${decimals} decimal${decimals === 1 ? "" : "s"} for ${symbol}.`
    };
  }

  const factor = 10n ** BigInt(decimals);
  const padded = decimals > 0 ? (frac + "0".repeat(decimals)).slice(0, decimals) : "";
  const raw = BigInt(whole) * factor + (decimals > 0 ? BigInt(padded) : 0n);

  if (raw <= 0n) return { kind: "error", message: "Amount must be greater than zero." };

  if (balanceRaw === null) return { kind: "ok", raw };

  if (raw > balanceRaw) {
    return {
      kind: "error",
      message: `Exceeds balance — max ${formatRaw(balanceRaw, decimals)} ${symbol}`
    };
  }

  if (feeReserve > 0n && raw + feeReserve > balanceRaw) {
    return {
      kind: "warn",
      raw,
      message: `Leave ~${formatRaw(feeReserve, decimals)} ${symbol} for the fee.`
    };
  }

  return { kind: "ok", raw };
}

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

export function useRecipientCheck(
  recipient: string,
  ownPubkey: string | null,
  intendedRaw: bigint | null
): RecipientCheck {
  const trimmed = recipient.trim();
  const formatValid = trimmed.length > 0 && isValidSolanaAddress(trimmed);

  const { data, error, isLoading } = useSWR(
    formatValid ? ["recipient-info", trimmed] : null,
    async ([, addr]: readonly [string, string]) => {
      const info = await solanaRpc
        .getAccountInfo(toAddress(addr) as Address, { encoding: "base64" })
        .send();
      return info.value;
    },
    {
      revalidateOnFocus: false,
      dedupingInterval: 10_000,
      shouldRetryOnError: false,
      keepPreviousData: false
    }
  );

  if (trimmed.length === 0 || !formatValid) return { kind: "ok" };

  if (ownPubkey && trimmed === ownPubkey) {
    return { kind: "warn", message: "This is your own address." };
  }

  if (isLoading) return { kind: "loading" };
  if (error) return { kind: "ok" };

  const accountInfo = data;

  if (accountInfo && accountInfo.executable) {
    return { kind: "error", message: "Recipient is a program — funds would be locked." };
  }

  if (accountInfo === null) {
    if (intendedRaw !== null && intendedRaw > 0n && intendedRaw < RENT_EXEMPT_MIN_LAMPORTS) {
      return {
        kind: "warn",
        message: `Fresh accounts need ≥ ${formatRaw(RENT_EXEMPT_MIN_LAMPORTS, 9)} SOL to stay alive.`
      };
    }
    return { kind: "ok", note: "Fresh address — first transaction here." };
  }

  return { kind: "ok" };
}

function formatRaw(raw: bigint, decimals: number): string {
  if (decimals === 0) return raw.toString();
  const factor = 10n ** BigInt(decimals);
  const whole = raw / factor;
  const frac = raw % factor;
  if (frac === 0n) return whole.toString();
  const fracStr = frac.toString().padStart(decimals, "0").replace(/0+$/, "");
  return `${whole.toString()}.${fracStr}`;
}
