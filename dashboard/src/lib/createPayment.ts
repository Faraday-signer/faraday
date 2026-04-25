/**
 * Build a Squads vault payment as a single transaction:
 *   1. `vaultTransactionCreate` — stores the inner transfer that the vault
 *      will execute once approved
 *   2. `proposalCreate`         — creates the proposal lifecycle account
 *   3. `proposalApprove`        — auto-counts the creator's own vote when
 *      they're a member with vote permission
 *
 * The user (HR / proposer) signs once; the proposal lands on-chain and
 * approvers can act on it from their dashboards.
 */

import {
  PublicKey,
  SystemProgram,
  TransactionMessage,
  VersionedTransaction,
} from "@solana/web3.js";
import * as multisig from "@sqds/multisig";
import { connection } from "./squads";

export interface PaymentInput {
  readonly multisigPda: PublicKey;
  readonly creator: PublicKey;                                    // pays tx fees + rent
  readonly transfers: ReadonlyArray<{ recipient: PublicKey; lamports: number }>;
  readonly memo?: string;
}

export interface PaymentPlan {
  readonly tx: VersionedTransaction;
  readonly transactionIndex: bigint;
  readonly vaultPda: PublicKey;
  readonly proposalPda: PublicKey;
  readonly autoApproved: boolean;
  readonly blockhash: string;
  readonly lastValidBlockHeight: number;
}

export async function planCreatePayment(input: PaymentInput): Promise<PaymentPlan> {
  if (input.transfers.length === 0) throw new Error("at least one recipient");
  if (input.transfers.some((t) => t.lamports <= 0)) {
    throw new Error("each amount must be greater than zero");
  }

  // The next transactionIndex is current + 1; Squads enforces strict sequencing.
  const ms = await multisig.accounts.Multisig.fromAccountAddress(connection, input.multisigPda);
  const transactionIndex = BigInt(Number(ms.transactionIndex) + 1);
  const isMember = ms.members.some((m) => m.key.equals(input.creator));
  const hasInitiate = isMember && ms.members.some(
    (m) => m.key.equals(input.creator) && (m.permissions.mask & 0x01) !== 0,
  );
  if (!hasInitiate) {
    throw new Error("Connected wallet doesn't have Initiate permission on this multisig.");
  }

  const [vaultPda] = multisig.getVaultPda({ multisigPda: input.multisigPda, index: 0 });
  const [proposalPda] = multisig.getProposalPda({
    multisigPda: input.multisigPda,
    transactionIndex,
  });

  // Inner transaction: what the vault executes once the proposal passes.
  // One SystemProgram.transfer instruction per recipient — Squads' vault
  // transaction can hold any number, capped only by Solana's tx size.
  // Placeholder blockhash; Squads stores the message body and rebinds
  // it to a fresh blockhash at execute time.
  const transferIxs = input.transfers.map((t) =>
    SystemProgram.transfer({
      fromPubkey: vaultPda,
      toPubkey: t.recipient,
      lamports: t.lamports,
    }),
  );
  const innerMessage = new TransactionMessage({
    payerKey: vaultPda,
    recentBlockhash: PublicKey.default.toBase58(),
    instructions: transferIxs,
  });

  // Wrapper instructions: create the proposal record + approve in one tx.
  const ixs = [
    multisig.instructions.vaultTransactionCreate({
      multisigPda: input.multisigPda,
      transactionIndex,
      creator: input.creator,
      rentPayer: input.creator,
      vaultIndex: 0,
      ephemeralSigners: 0,
      transactionMessage: innerMessage,
      memo: input.memo,
    }),
    multisig.instructions.proposalCreate({
      multisigPda: input.multisigPda,
      transactionIndex,
      creator: input.creator,
      rentPayer: input.creator,
    }),
  ];
  const canVote = isMember && ms.members.some(
    (m) => m.key.equals(input.creator) && (m.permissions.mask & 0x02) !== 0,
  );
  if (canVote) {
    ixs.push(multisig.instructions.proposalApprove({
      multisigPda: input.multisigPda,
      transactionIndex,
      member: input.creator,
    }));
  }

  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
  const message = new TransactionMessage({
    payerKey: input.creator,
    recentBlockhash: blockhash,
    instructions: ixs,
  }).compileToV0Message();
  const tx = new VersionedTransaction(message);

  return {
    tx,
    transactionIndex,
    vaultPda,
    proposalPda,
    autoApproved: canVote,
    blockhash,
    lastValidBlockHeight,
  };
}
