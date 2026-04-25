/**
 * Build the three approver-side transactions:
 *   - approve  (proposalApprove)
 *   - reject   (proposalReject)
 *   - execute  (vaultTransactionExecute, only valid once threshold reached)
 *
 * Each one is a single-instruction tx signed by the connected member.
 */

import {
  PublicKey,
  TransactionMessage,
  VersionedTransaction,
} from "@solana/web3.js";
import * as multisig from "@sqds/multisig";
import { connection } from "./squads";

export interface VoteInput {
  readonly multisigPda: PublicKey;
  readonly transactionIndex: bigint;
  readonly member: PublicKey;
}

export interface VotePlan {
  readonly tx: VersionedTransaction;
  readonly blockhash: string;
  readonly lastValidBlockHeight: number;
}

export async function planApprove(input: VoteInput): Promise<VotePlan> {
  return wrap(input.member, [
    multisig.instructions.proposalApprove({
      multisigPda: input.multisigPda,
      transactionIndex: input.transactionIndex,
      member: input.member,
    }),
  ]);
}

export async function planReject(input: VoteInput): Promise<VotePlan> {
  return wrap(input.member, [
    multisig.instructions.proposalReject({
      multisigPda: input.multisigPda,
      transactionIndex: input.transactionIndex,
      member: input.member,
    }),
  ]);
}

export async function planExecute(input: VoteInput): Promise<VotePlan> {
  const { instruction, lookupTableAccounts } = await multisig.instructions.vaultTransactionExecute({
    connection,
    multisigPda: input.multisigPda,
    transactionIndex: input.transactionIndex,
    member: input.member,
  });

  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
  const message = new TransactionMessage({
    payerKey: input.member,
    recentBlockhash: blockhash,
    instructions: [instruction],
  }).compileToV0Message(lookupTableAccounts);
  return { tx: new VersionedTransaction(message), blockhash, lastValidBlockHeight };
}

async function wrap(payer: PublicKey, ixs: ReturnType<typeof multisig.instructions.proposalApprove>[]): Promise<VotePlan> {
  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
  const message = new TransactionMessage({
    payerKey: payer,
    recentBlockhash: blockhash,
    instructions: ixs,
  }).compileToV0Message();
  return { tx: new VersionedTransaction(message), blockhash, lastValidBlockHeight };
}
