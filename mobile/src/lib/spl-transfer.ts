import {
  address,
  appendTransactionMessageInstructionPlan,
  compileTransaction,
  createNoopSigner,
  createTransactionMessage,
  getBase64EncodedWireTransaction,
  pipe,
  setTransactionMessageFeePayer,
  setTransactionMessageLifetimeUsingBlockhash
} from "@solana/kit";
import { getTransferToATAInstructionPlanAsync } from "@solana-program/token";

import { solanaRpc } from "./sol-client";

export interface BuildSplTransferInput {
  /** Paired pubkey (fee payer + source authority). */
  from: string;
  /** Recipient wallet (not their ATA — the ATA is derived). */
  to: string;
  /** Mint address (base58). */
  mint: string;
  /** Mint decimals — used by transferChecked. */
  decimals: number;
  /** Amount in atoms (raw integer). */
  amountRaw: bigint;
  /** Source ATA — must already exist. Caller derives or pulls from token list. */
  sourceAta: string;
}

/**
 * Build an unsigned SPL token transfer as a base64 legacy transaction.
 *
 * Uses `getTransferToATAInstructionPlanAsync` which:
 *   1. Derives the recipient's associated token account.
 *   2. Inserts a `createAssociatedTokenAccount` instruction if the
 *      destination ATA doesn't exist yet (the fee payer pays the rent).
 *   3. Adds a `transferChecked` instruction with the supplied decimals.
 *
 * The returned plan is appended to a fresh tx message via
 * `appendTransactionMessageInstructionPlan` so any future multi-instruction
 * plans (priority fees, ATA close, etc.) can be added the same way.
 */
export async function buildSplTransfer(
  input: BuildSplTransferInput
): Promise<{ txBase64: string; amountRaw: bigint }> {
  if (input.amountRaw <= 0n) {
    throw new Error("Amount must be greater than zero.");
  }

  const from = address(input.from);
  const to = address(input.to);
  const mint = address(input.mint);
  const sourceAta = address(input.sourceAta);

  const { value: latestBlockhash } = await solanaRpc
    .getLatestBlockhash({ commitment: "finalized" })
    .send();

  const payer = createNoopSigner(from);

  const plan = await getTransferToATAInstructionPlanAsync({
    payer,
    mint,
    source: sourceAta,
    authority: payer,
    recipient: to,
    amount: input.amountRaw,
    decimals: input.decimals
  });

  const message = pipe(
    createTransactionMessage({ version: "legacy" }),
    (m) => setTransactionMessageFeePayer(from, m),
    (m) => setTransactionMessageLifetimeUsingBlockhash(latestBlockhash, m),
    (m) => appendTransactionMessageInstructionPlan(plan, m)
  );

  const compiled = compileTransaction(message);
  const txBase64 = getBase64EncodedWireTransaction(compiled);

  return { txBase64, amountRaw: input.amountRaw };
}
