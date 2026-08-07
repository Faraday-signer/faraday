//! SOL transfer construction + broadcast for the sidepanel send flow.
//!
//! Uses `@solana/kit` + `@solana-program/system`. The unsigned tx is
//! serialized in legacy wire format (1 signature slot, all-zero) so the
//! Faraday firmware parses it the same way dapp-originated txs do.
//!
//! Every transfer Faraday builds uses a **durable nonce** (owner decision,
//! cxalem): the tx leads with `AdvanceNonceAccount` and pins its lifetime to
//! the wallet's nonce account, so the signature can't expire during a slow QR
//! relay. The nonce account is provisioned once per wallet (see `nonce.ts`);
//! until then `buildSolTransfer` throws `NonceAccountNotProvisionedError` and
//! the send flow provisions before continuing.
//!
//! Flow on the sidepanel side:
//!   buildSolTransfer  → base64 unsigned durable-nonce tx
//!   (user scans with Faraday → firmware signs → user scans back in sign window)
//!   broadcastSignedTx → { signature, explorerUrl }

import { CLUSTER_ID, solanaRpc } from "./sol-client";
import {
  buildDurableNonceTransferTx,
  fetchNonceValue,
} from "./nonce";
import { getNonceAccount } from "./storage";

export const LAMPORTS_PER_SOL = 1_000_000_000n;

/**
 * Thrown by `buildSolTransfer` when the wallet has no durable-nonce account
 * yet. The send flow catches this to run the one-time provisioning step
 * before building the transfer.
 */
export class NonceAccountNotProvisionedError extends Error {
  constructor() {
    super("No durable-nonce account provisioned for this wallet.");
    this.name = "NonceAccountNotProvisionedError";
  }
}

export interface BuildSolTransferInput {
  /** Paired pubkey (fee payer + source). */
  from: string;
  /** Recipient base58 address. */
  to: string;
  /** Amount in SOL as a user-facing decimal string (e.g. "0.25"). */
  amountSol: string;
}

export interface BroadcastResult {
  signature: string;
  explorerUrl: string;
}

/**
 * Parse a user-entered SOL amount string into lamports. Rejects negatives,
 * NaN, and anything with more than 9 decimal places. Avoids Number float
 * rounding by doing the decimal shift as string ops.
 */
export function amountToLamports(amountSol: string): bigint {
  const trimmed = amountSol.trim();
  if (!/^\d+(\.\d+)?$/.test(trimmed)) {
    throw new Error("Amount must be a positive decimal number.");
  }
  const [whole, frac = ""] = trimmed.split(".");
  if (frac.length > 9) {
    throw new Error("Max precision is 9 decimals (1 lamport).");
  }
  const padded = (frac + "000000000").slice(0, 9);
  const lamports = BigInt(whole) * LAMPORTS_PER_SOL + BigInt(padded);
  if (lamports <= 0n) {
    throw new Error("Amount must be greater than zero.");
  }
  return lamports;
}

/**
 * Build an unsigned durable-nonce SOL transfer as a base64 legacy transaction.
 * Resolves the wallet's nonce account (persisted at provisioning time) and
 * reads its current nonce value, then builds a transfer that leads with
 * `AdvanceNonceAccount`. One signer slot — the wallet. Returns the base64 (for
 * the sign session payload) and the raw lamports figure (for the review
 * screen).
 *
 * Throws `NonceAccountNotProvisionedError` if the wallet has no nonce account
 * yet, so the caller can provision one first.
 */
export async function buildSolTransfer(
  input: BuildSolTransferInput
): Promise<{ txBase64: string; lamports: bigint }> {
  const lamports = amountToLamports(input.amountSol);

  const nonceAccountAddress = await getNonceAccount(input.from);
  if (!nonceAccountAddress) {
    throw new NonceAccountNotProvisionedError();
  }
  const nonceValue = await fetchNonceValue(nonceAccountAddress);

  const txBase64 = buildDurableNonceTransferTx({
    from: input.from,
    to: input.to,
    lamports,
    nonceAccountAddress,
    nonceValue,
  });

  return { txBase64, lamports };
}

/**
 * Broadcast a signed base64 tx. Returns the signature + a Solana-Explorer
 * URL pinned to the configured cluster so the sidepanel can surface a
 * clickable link immediately — confirmation polling is caller's choice.
 */
export async function broadcastSignedTx(
  signedTxBase64: string
): Promise<BroadcastResult> {
  const signature = await solanaRpc
    .sendTransaction(signedTxBase64 as any, {
      encoding: "base64",
      skipPreflight: false,
      preflightCommitment: "confirmed",
      maxRetries: 3n,
    })
    .send();

  return {
    signature: String(signature),
    explorerUrl: explorerTxUrl(String(signature)),
  };
}

/** Build a Solana-Explorer URL for a signature on the configured cluster. */
export function explorerTxUrl(signature: string): string {
  const cluster = CLUSTER_ID === "mainnet-beta" ? "" : `?cluster=${CLUSTER_ID}`;
  return `https://explorer.solana.com/tx/${signature}${cluster}`;
}
