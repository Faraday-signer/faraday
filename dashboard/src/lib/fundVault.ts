/**
 * Build a plain SystemProgram.transfer to send SOL into a multisig's vault.
 * No Squads wrapping — anyone can credit a vault PDA, no approval flow.
 */

import {
  PublicKey,
  SystemProgram,
  TransactionMessage,
  VersionedTransaction,
} from "@solana/web3.js";
import { connection } from "./squads";

export interface FundVaultInput {
  readonly from: PublicKey;
  readonly vault: PublicKey;
  readonly lamports: number;
}

export interface FundVaultPlan {
  readonly tx: VersionedTransaction;
  readonly blockhash: string;
  readonly lastValidBlockHeight: number;
}

export async function planFundVault(input: FundVaultInput): Promise<FundVaultPlan> {
  if (input.lamports <= 0) throw new Error("amount must be greater than zero");

  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
  const ix = SystemProgram.transfer({
    fromPubkey: input.from,
    toPubkey: input.vault,
    lamports: input.lamports,
  });
  const message = new TransactionMessage({
    payerKey: input.from,
    recentBlockhash: blockhash,
    instructions: [ix],
  }).compileToV0Message();
  return { tx: new VersionedTransaction(message), blockhash, lastValidBlockHeight };
}
