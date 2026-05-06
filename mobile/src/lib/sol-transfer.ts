import {
  address,
  appendTransactionMessageInstruction,
  compileTransaction,
  createTransactionMessage,
  getBase64EncodedWireTransaction,
  pipe,
  setTransactionMessageFeePayer,
  setTransactionMessageLifetimeUsingBlockhash
} from "@solana/kit";
import { getTransferSolInstruction } from "@solana-program/system";

import { CLUSTER_ID, solanaRpc } from "./sol-client";

export const LAMPORTS_PER_SOL = 1_000_000_000n;

export interface BuildSolTransferInput {
  from: string;
  to: string;
  amountSol: string;
}

export interface BroadcastResult {
  signature: string;
  explorerUrl: string;
}

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
    amount: lamports
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

export async function broadcastSignedTx(signedTxBase64: string): Promise<BroadcastResult> {
  const signature = await solanaRpc
    .sendTransaction(signedTxBase64 as any, {
      encoding: "base64",
      skipPreflight: false,
      preflightCommitment: "confirmed",
      maxRetries: 3n
    })
    .send();

  return {
    signature: String(signature),
    explorerUrl: explorerTxUrl(String(signature))
  };
}

export function explorerTxUrl(signature: string): string {
  const cluster = CLUSTER_ID === "mainnet-beta" ? "" : `?cluster=${CLUSTER_ID}`;
  return `https://explorer.solana.com/tx/${signature}${cluster}`;
}
