//! Durable-nonce rewrite for dapp-built transactions.
//!
//! Revises the FA-09 decision recorded in `nonce.ts`: "Dapp-built
//! transaction messages are never modified." FA-09 only covered
//! transactions Faraday itself builds (the Send flow); dapp-built
//! transactions (Jupiter swaps, playground demos, the Ika approver flow)
//! stayed pinned to a perishable blockhash — and those are exactly the
//! largest, slowest-relaying animated-QR transactions, the ones most
//! likely to expire mid-scan. This module rewrites a dapp's unsigned
//! transaction into durable-nonce form *when it's safe to do so*, and
//! falls back to byte-exact passthrough on every unsafe case. A dapp sign
//! must never hard-fail because of this rewrite.
//!
//! `rewriteDappTxToDurableNonce` never throws. Gates run cheapest-first,
//! with no RPC call before the wallet is confirmed to have a nonce account:
//!   1. Any non-zero (partial) signature already present → passthrough.
//!      Rewriting would change the message bytes a signature was already
//!      collected over.
//!   2. Fee payer isn't the paired wallet → passthrough. The durable-nonce
//!      authority is always the wallet; we never add a new required signer.
//!   3. The message already leads with `AdvanceNonceAccount` → passthrough
//!      (`"already-durable-nonce"`), nothing to do.
//!   4. No nonce account provisioned for this wallet → passthrough
//!      (`"no-nonce-account"`). The sign popup shows a one-time setup
//!      interstitial for this case (see `sign-app.tsx`) and retries the
//!      rewrite once provisioning completes.
//!   5. Decompile the message (fetching address-lookup-table contents over
//!      RPC only when the message references any), set the durable-nonce
//!      lifetime, recompile, and check the result still fits in a
//!      transaction packet. Any RPC failure, timeout, or unexpected kit
//!      exception here also falls back to passthrough.
//!
//! Deliberately **not** calling
//! `compressTransactionMessageUsingAddressLookupTables` after the rewrite:
//! it could fold the nonce account itself into a dapp-supplied lookup
//! table, and kit has a dedicated error for exactly that hazard.
//!
//! Known limitation (same as FA-09): one nonce account per wallet, so two
//! dapp transactions signed concurrently race the same nonce — one of them
//! will fail to land. A future nonce pool could remove this.

import {
  address,
  compileTransaction,
  decompileTransactionMessage,
  fetchAddressesForLookupTables,
  getBase64EncodedWireTransaction,
  getCompiledTransactionMessageDecoder,
  getTransactionDecoder,
  getTransactionSize,
  setTransactionMessageLifetimeUsingDurableNonce,
  TRANSACTION_SIZE_LIMIT,
  type Nonce,
} from "@solana/kit";
import { SYSTEM_PROGRAM_ADDRESS } from "@solana-program/system";

import { fetchNonceValue } from "./nonce";
import { decodeBase64, parseEnvelope } from "./solana";
import { solanaRpc } from "./sol-client";
import { getNonceAccount } from "./storage";

const DEFAULT_TIMEOUT_MS = 4_000;
const ADVANCE_NONCE_ACCOUNT_DISCRIMINANT = 4;

export type NonceRewriteReason =
  | "rewritten"
  | "disabled"
  | "has-partial-signatures"
  | "fee-payer-not-wallet"
  | "already-durable-nonce"
  | "no-nonce-account"
  | "oversize"
  | "timeout"
  | "rewrite-failed";

export interface NonceRewriteResult {
  txBase64: string;
  rewritten: boolean;
  reason: NonceRewriteReason;
}

type CompiledMessage = ReturnType<
  ReturnType<typeof getCompiledTransactionMessageDecoder>["decode"]
>;

function passthrough(txBase64: string, reason: NonceRewriteReason): NonceRewriteResult {
  return { txBase64, rewritten: false, reason };
}

function allZero(bytes: Uint8Array): boolean {
  return bytes.every((byte) => byte === 0);
}

function isAdvanceNonceAccountData(data: ArrayLike<number> | undefined): boolean {
  if (!data || data.length < 4) return false;
  return (
    data[0] === ADVANCE_NONCE_ACCOUNT_DISCRIMINANT &&
    data[1] === 0 &&
    data[2] === 0 &&
    data[3] === 0
  );
}

/** Read-only check: does the compiled message already lead with a System `AdvanceNonceAccount`? */
function alreadyDurableNonce(compiled: CompiledMessage): boolean {
  const first = compiled.instructions[0];
  if (!first) return false;
  const programAddress = compiled.staticAccounts[first.programAddressIndex];
  return programAddress === SYSTEM_PROGRAM_ADDRESS && isAdvanceNonceAccountData(first.data);
}

class NonceRewriteTimeoutError extends Error {}

function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new NonceRewriteTimeoutError()), ms);
    promise.then(
      (value) => {
        clearTimeout(timer);
        resolve(value);
      },
      (error) => {
        clearTimeout(timer);
        reject(error);
      }
    );
  });
}

/**
 * Fetch the current nonce value and (only if the message references any)
 * the contents of every address lookup table it uses, in parallel, then
 * decompile → set the durable-nonce lifetime → recompile. Returns an
 * `"oversize"` passthrough if the rewritten transaction no longer fits in
 * a packet; any other failure propagates to the caller's timeout/try-catch.
 */
async function runRewritePipeline(
  compiled: CompiledMessage,
  originalTxBase64: string,
  walletPubkey: string,
  nonceAccountAddress: string
): Promise<NonceRewriteResult> {
  const lookups = compiled.version === 0 ? (compiled.addressTableLookups ?? []) : [];

  const [nonceValue, addressesByLookupTableAddress] = await Promise.all([
    fetchNonceValue(nonceAccountAddress),
    lookups.length > 0
      ? fetchAddressesForLookupTables(
          lookups.map((lookup) => lookup.lookupTableAddress),
          solanaRpc
        )
      : Promise.resolve(undefined),
  ]);

  const decompiled = decompileTransactionMessage(compiled, {
    addressesByLookupTableAddress,
  });

  const withDurableNonce = setTransactionMessageLifetimeUsingDurableNonce(
    {
      nonce: nonceValue as Nonce,
      nonceAccountAddress: address(nonceAccountAddress),
      nonceAuthorityAddress: address(walletPubkey),
    },
    decompiled
  );

  const rewrittenTx = compileTransaction(withDurableNonce);
  if (getTransactionSize(rewrittenTx) > TRANSACTION_SIZE_LIMIT) {
    return passthrough(originalTxBase64, "oversize");
  }

  return {
    txBase64: getBase64EncodedWireTransaction(rewrittenTx),
    rewritten: true,
    reason: "rewritten",
  };
}

/**
 * Rewrite a dapp-built unsigned transaction into durable-nonce form when
 * it's safe to do so. Never throws — every failure mode returns the
 * original `txBase64` unchanged, tagged with the reason it wasn't
 * rewritten, so the caller can log it and proceed with the original bytes.
 */
export async function rewriteDappTxToDurableNonce(
  txBase64: string,
  walletPubkey: string,
  opts: { timeoutMs?: number } = {}
): Promise<NonceRewriteResult> {
  const timeoutMs = opts.timeoutMs ?? DEFAULT_TIMEOUT_MS;

  let txBytes: Uint8Array;
  let compiled: CompiledMessage;
  try {
    txBytes = decodeBase64(txBase64.trim());

    // Gate 1 + 2: any partial signature, or a fee payer that isn't the
    // paired wallet, disqualifies the rewrite. Both read straight off the
    // wire envelope — no RPC.
    const envelope = parseEnvelope(txBytes);
    if (envelope.signatures.some((sig) => !allZero(sig))) {
      return passthrough(txBase64, "has-partial-signatures");
    }
    if (envelope.signerAddresses[0] !== walletPubkey) {
      return passthrough(txBase64, "fee-payer-not-wallet");
    }

    // Gate 3: already durable-nonce. Decoded separately via kit so the
    // rewrite pipeline below can reuse the same `CompiledMessage`.
    const transaction = getTransactionDecoder().decode(txBytes);
    compiled = getCompiledTransactionMessageDecoder().decode(transaction.messageBytes);
    if (alreadyDurableNonce(compiled)) {
      return passthrough(txBase64, "already-durable-nonce");
    }
  } catch {
    return passthrough(txBase64, "rewrite-failed");
  }

  // Gate 4: no nonce account provisioned yet. Still no RPC — `getNonceAccount`
  // reads local extension storage.
  const nonceAccountAddress = await getNonceAccount(walletPubkey);
  if (!nonceAccountAddress) {
    return passthrough(txBase64, "no-nonce-account");
  }

  // Gate 5: the actual rewrite, under a hard deadline. Any RPC failure or
  // unexpected kit exception (e.g. an account that's both a static signer
  // and a lookup-table entry) falls back to passthrough rather than
  // surfacing to the caller.
  try {
    return await withTimeout(
      runRewritePipeline(compiled, txBase64, walletPubkey, nonceAccountAddress),
      timeoutMs
    );
  } catch (error) {
    return passthrough(txBase64, error instanceof NonceRewriteTimeoutError ? "timeout" : "rewrite-failed");
  }
}
