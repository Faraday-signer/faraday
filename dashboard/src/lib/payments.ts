/**
 * Read pending (and recently active) payments for a multisig.
 *
 * Squads stores each proposed payment as a pair of accounts:
 *   - VaultTransaction (PDA at index N) — holds the inner instructions
 *   - Proposal       (PDA at index N) — holds the lifecycle state
 * The multisig itself tracks `transactionIndex` (highest used) and
 * `staleTransactionIndex` (anything ≤ this is no longer executable).
 */

import { PublicKey, SystemProgram } from "@solana/web3.js";
import * as multisig from "@sqds/multisig";
import { connection } from "./squads";

export type PaymentStatus =
  | "Draft" | "Active" | "Approved" | "Executing"
  | "Executed" | "Rejected" | "Cancelled";

export interface Transfer {
  readonly recipient: string;
  readonly lamports: number;
}

export interface PendingPayment {
  readonly transactionIndex: number;
  readonly status: PaymentStatus;
  readonly approvedBy: ReadonlyArray<string>;
  readonly rejectedBy: ReadonlyArray<string>;
  readonly cancelledBy: ReadonlyArray<string>;
  readonly transfers: ReadonlyArray<Transfer>;
  /** Instructions we couldn't decode (non SystemProgram.transfer). */
  readonly otherInstructions: number;
  readonly proposalPda: string;
  readonly transactionPda: string;
  readonly createdAt: number | null;  // unix seconds, when known
}

const SYSTEM_PROGRAM_B58 = SystemProgram.programId.toBase58();
const TERMINAL: ReadonlyArray<PaymentStatus> = ["Executed", "Rejected", "Cancelled"];

/**
 * Returns proposals that are still actionable (Active / Approved / Executing
 * / Draft). Terminal states (Executed/Rejected/Cancelled) are filtered out.
 */
export async function fetchPendingPayments(multisigPda: PublicKey): Promise<PendingPayment[]> {
  const all = await fetchAllPayments(multisigPda);
  return all.filter((r) => !TERMINAL.includes(r.status));
}

/**
 * Returns *every* payment proposal (including executed / rejected / cancelled)
 * since the multisig's stale-tx threshold. Used by the Payments page to
 * surface a full history with status badges.
 */
export async function fetchAllPayments(multisigPda: PublicKey): Promise<PendingPayment[]> {
  const ms = await multisig.accounts.Multisig.fromAccountAddress(connection, multisigPda);
  const stale = Number(ms.staleTransactionIndex);
  const current = Number(ms.transactionIndex);
  if (current <= stale) return [];

  const indices: number[] = [];
  for (let i = stale + 1; i <= current; i++) indices.push(i);

  const rows = await Promise.all(indices.map((i) => fetchOne(multisigPda, i)));
  return rows
    .filter((r): r is PendingPayment => r !== null)
    .sort((a, b) => b.transactionIndex - a.transactionIndex); // newest first
}

async function fetchOne(multisigPda: PublicKey, index: number): Promise<PendingPayment | null> {
  const txIdx = BigInt(index);
  const [proposalPda] = multisig.getProposalPda({ multisigPda, transactionIndex: txIdx });
  const [transactionPda] = multisig.getTransactionPda({ multisigPda, index: txIdx });

  const [proposal, vaultTx] = await Promise.all([
    multisig.accounts.Proposal.fromAccountAddress(connection, proposalPda).catch(() => null),
    multisig.accounts.VaultTransaction.fromAccountAddress(connection, transactionPda).catch(() => null),
  ]);
  if (!proposal || !vaultTx) return null;

  const status = proposal.status.__kind as PaymentStatus;
  const createdAt = "timestamp" in proposal.status
    ? Number((proposal.status as unknown as { timestamp: bigint | number }).timestamp)
    : null;

  const decoded = decodeTransfers(vaultTx.message);

  return {
    transactionIndex: index,
    status,
    approvedBy: proposal.approved.map((k) => k.toBase58()),
    rejectedBy: proposal.rejected.map((k) => k.toBase58()),
    cancelledBy: proposal.cancelled.map((k) => k.toBase58()),
    transfers: decoded.transfers,
    otherInstructions: decoded.others,
    proposalPda: proposalPda.toBase58(),
    transactionPda: transactionPda.toBase58(),
    createdAt,
  };
}

interface MultisigMessage {
  accountKeys: PublicKey[];
  instructions: ReadonlyArray<{
    programIdIndex: number;
    accountIndexes: Uint8Array;
    data: Uint8Array;
  }>;
}

function decodeTransfers(message: MultisigMessage): { transfers: Transfer[]; others: number } {
  const keys = message.accountKeys.map((k) => k.toBase58());
  const transfers: Transfer[] = [];
  let others = 0;

  for (const ix of message.instructions) {
    const programId = keys[ix.programIdIndex];
    if (programId !== SYSTEM_PROGRAM_B58) { others += 1; continue; }

    // SystemProgram.transfer: u32 LE tag (=2) + u64 LE lamports
    const data = ix.data;
    if (data.length !== 12) { others += 1; continue; }
    const tag = readU32LE(data, 0);
    if (tag !== 2) { others += 1; continue; }
    const lamports = Number(readU64LE(data, 4));

    if (ix.accountIndexes.length < 2) { others += 1; continue; }
    const recipient = keys[ix.accountIndexes[1]];
    transfers.push({ recipient, lamports });
  }
  return { transfers, others };
}

function readU32LE(b: Uint8Array, off: number): number {
  return ((b[off] | (b[off + 1] << 8) | (b[off + 2] << 16)) >>> 0) +
    b[off + 3] * 0x01000000;
}

function readU64LE(b: Uint8Array, off: number): bigint {
  // Workers / browsers without BigInt64Array support: assemble manually.
  let lo = 0;
  let hi = 0;
  for (let i = 0; i < 4; i++) lo |= b[off + i] << (i * 8);
  for (let i = 0; i < 4; i++) hi |= b[off + 4 + i] << (i * 8);
  return (BigInt(hi >>> 0) << 32n) | BigInt(lo >>> 0);
}
