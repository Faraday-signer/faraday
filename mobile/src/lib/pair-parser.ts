import { isValidSolanaAddress } from "./solana";

export type PairScanResult =
  | { kind: "pair"; pubkey: string }
  | { kind: "wrong-mode"; hint: string }
  | { kind: "invalid" };

/**
 * Parse a pair-flow input. Accepts the Pi's `faraday:pair:<address>` envelope,
 * `solana:<address>` URIs, and bare base58 addresses. Recognises sign/tx-mode
 * QRs and routes the user to the correct device screen.
 */
export function parsePairInput(raw: string): PairScanResult {
  const trimmed = raw.trim();
  if (!trimmed) return { kind: "invalid" };
  const lower = trimmed.toLowerCase();

  if (lower.startsWith("faraday:pair:")) {
    const candidate = trimmed.slice("faraday:pair:".length).split(/[?&#]/)[0];
    return isValidSolanaAddress(candidate)
      ? { kind: "pair", pubkey: candidate }
      : { kind: "invalid" };
  }

  if (lower.startsWith("faraday:sig:") || lower.startsWith("faraday:signed:")) {
    return {
      kind: "wrong-mode",
      hint: "That's a signed-transaction QR. On your Faraday, go to Home → Show Address."
    };
  }

  if (lower.startsWith("faraday:unsigned:") || lower.startsWith("faraday:tx:")) {
    return {
      kind: "wrong-mode",
      hint: "That's a transaction QR, not a wallet QR. On your Faraday, go to Home → Show Address."
    };
  }

  if (lower.startsWith("solana:")) {
    const candidate = trimmed.slice("solana:".length).split(/[?&#]/)[0];
    if (isValidSolanaAddress(candidate)) {
      return { kind: "pair", pubkey: candidate };
    }
  }

  if (isValidSolanaAddress(trimmed)) {
    return { kind: "pair", pubkey: trimmed };
  }

  return { kind: "invalid" };
}
