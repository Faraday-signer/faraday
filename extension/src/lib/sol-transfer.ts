//! SOL transfer construction + broadcast for the sidepanel send flow.
//!
//! Uses `@solana/kit` + `@solana-program/system`. The unsigned tx is
//! serialized in legacy wire format (1 signature slot, all-zero) so the
//! Faraday firmware parses it the same way dapp-originated txs do.
//!
//! Flow on the sidepanel side:
//!   buildSolTransfer  → base64 unsigned tx
//!   (user scans with Faraday → firmware signs → user scans back in sign window)
//!   broadcastSignedTx → { signature, explorerUrl }

import {
  address,
  appendTransactionMessageInstruction,
  compileTransaction,
  createTransactionMessage,
  getBase64EncodedWireTransaction,
  pipe,
  setTransactionMessageFeePayer,
  setTransactionMessageLifetimeUsingBlockhash,
} from "@solana/kit";
import { getTransferSolInstruction } from "@solana-program/system";

import { CLUSTER_ID, solanaRpc } from "./sol-client";

export const LAMPORTS_PER_SOL = 1_000_000_000n;

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
 * Build an unsigned SOL transfer as a base64 legacy transaction. The fee
 * payer is the source (standard single-signer case), so there's one sig
 * slot to fill. Returns both the base64 (for the sign session payload)
 * and the raw lamports figure (for the review screen).
 */
export async function buildSolTransfer(
  input: BuildSolTransferInput
): Promise<{ txBase64: string; lamports: bigint }> {
  const lamports = amountToLamports(input.amountSol);

  const from = address(input.from);
  const to = address(input.to);

  const { value: latestBlockhash } = await solanaRpc
    .getLatestBlockhash({ commitment: "finalized" })
    .send();

  const transferIx = getTransferSolInstruction({
    source: { address: from, role: 3 } as any,
    destination: to,
    amount: lamports,
  });

  const message = pipe(
    createTransactionMessage({ version: "legacy" }),
    (m) => setTransactionMessageFeePayer(from, m),
    (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
    (m) => appendTransactionMessageInstruction(transferIx, m)
  );

  const compiled = compileTransaction(message);
  const txBase64 = getBase64EncodedWireTransaction(compiled);

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
