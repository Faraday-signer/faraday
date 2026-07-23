//! Durable-nonce transaction construction for Faraday-built transfers.
//!
//! Relaying QR codes between the browser and the device takes real time, and a
//! transaction pinned to a recent blockhash goes stale in ~60-90s. Every
//! transfer Faraday itself builds therefore uses a Solana durable nonce: the
//! transaction references a nonce account's stored blockhash and leads with a
//! `SystemProgram::AdvanceNonceAccount` instruction, so the signature stays
//! valid until the nonce next advances — however long the relay takes.
//!
//! Design (owner decision, cxalem): durable nonce **always, wherever Faraday
//! builds the transaction**. Dapp-built transaction messages are never
//! modified — altering them would break what the dapp expects to submit.
//!
//! Lifecycle:
//!   1. One-time: `buildCreateNonceAccountTx` funds + initializes a nonce
//!      account owned by the wallet (nonce authority = the wallet). The
//!      ephemeral nonce-account keypair signs the create tx alongside the
//!      wallet, then is discarded — only the address is persisted.
//!   2. Per transfer: `fetchNonceValue` reads the current nonce, then
//!      `buildDurableNonceTransferTx` builds the transfer with the leading
//!      advance instruction. If the nonce advanced between build and submit,
//!      the caller re-fetches and rebuilds (stale-nonce path).

import {
  address,
  appendTransactionMessageInstructions,
  compileTransaction,
  createNoopSigner,
  createTransactionMessage,
  generateKeyPairSigner,
  getBase64EncodedWireTransaction,
  partiallySignTransactionMessageWithSigners,
  pipe,
  setTransactionMessageFeePayer,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  setTransactionMessageLifetimeUsingDurableNonce,
  type Blockhash,
  type KeyPairSigner,
  type Nonce,
} from "@solana/kit";
import {
  fetchNonce,
  getCreateAccountInstruction,
  getInitializeNonceAccountInstruction,
  getTransferSolInstruction,
  SYSTEM_PROGRAM_ADDRESS,
} from "@solana-program/system";

import { solanaRpc } from "./sol-client";

/**
 * On-chain size of a System nonce account:
 *   version(4) + state(4) + authority(32) + stored blockhash(32) +
 *   fee_calculator(8) = 80 bytes.
 * Fixed by the runtime; used to size the funding + rent for creation.
 */
export const NONCE_ACCOUNT_SPACE = 80n;

export interface BuildDurableNonceTransferInput {
  /** Fee payer + source + nonce authority (the paired wallet). */
  from: string;
  /** Recipient base58 address. */
  to: string;
  /** Amount in lamports. */
  lamports: bigint;
  /** Base58 address of the wallet's nonce account. */
  nonceAccountAddress: string;
  /** Current nonce value (the nonce account's stored blockhash). */
  nonceValue: string;
}

/**
 * Build an unsigned durable-nonce SOL transfer as a base64 legacy transaction.
 *
 * `setTransactionMessageLifetimeUsingDurableNonce` prepends the
 * `AdvanceNonceAccount` instruction, so the compiled message leads with it and
 * pins its lifetime to the nonce value (not a perishable blockhash). One
 * signer slot — the wallet, which is both fee payer and nonce authority — so
 * the relay envelope is identical to today's single-signer transfer.
 */
export function buildDurableNonceTransferTx(
  input: BuildDurableNonceTransferInput
): string {
  const from = address(input.from);
  const to = address(input.to);
  const nonceAccountAddress = address(input.nonceAccountAddress);

  const transferIx = getTransferSolInstruction({
    source: { address: from, role: 3 } as never,
    destination: to,
    amount: input.lamports,
  });

  const message = pipe(
    createTransactionMessage({ version: "legacy" }),
    (m) => setTransactionMessageFeePayer(from, m),
    (m) =>
      setTransactionMessageLifetimeUsingDurableNonce(
        {
          nonce: input.nonceValue as Nonce,
          nonceAccountAddress,
          nonceAuthorityAddress: from,
        },
        m
      ),
    (m) => appendTransactionMessageInstructions([transferIx], m)
  );

  const compiled = compileTransaction(message);
  return getBase64EncodedWireTransaction(compiled);
}

export interface BuildCreateNonceAccountInput {
  /** Fee payer + nonce authority (the paired wallet). */
  payer: string;
  /** Rent-exempt minimum for an 80-byte nonce account, in lamports. */
  rentLamports: bigint;
  /** Recent blockhash for the create tx itself (this tx is short-lived). */
  recentBlockhash: {
    blockhash: Blockhash;
    lastValidBlockHeight: bigint;
  };
  /**
   * Test seam: inject the ephemeral nonce-account signer. Production callers
   * omit it and a fresh keypair is generated.
   */
  nonceSigner?: KeyPairSigner;
}

export interface CreateNonceAccountResult {
  /** Partially-signed (nonce keypair only) create tx, base64 legacy wire. */
  txBase64: string;
  /** Address of the nonce account being created. */
  nonceAccountAddress: string;
}

/**
 * Build the one-time nonce-account creation transaction: `CreateAccount`
 * (payer funds an 80-byte account owned by the System program) +
 * `InitializeNonceAccount` (authority = the wallet).
 *
 * Two signers: the wallet (fee payer, signed later on the device) and the
 * ephemeral nonce-account keypair. We sign the nonce-account slot here and
 * leave the wallet slot empty — the device fills it during the relay, and
 * `spliceFaradaySignature` preserves the pre-filled nonce signature. The
 * nonce keypair is used once and never persisted.
 */
export async function buildCreateNonceAccountTx(
  input: BuildCreateNonceAccountInput
): Promise<CreateNonceAccountResult> {
  const payerAddress = address(input.payer);
  const payerSigner = createNoopSigner(payerAddress);
  const nonceSigner = input.nonceSigner ?? (await generateKeyPairSigner());

  const createIx = getCreateAccountInstruction({
    payer: payerSigner,
    newAccount: nonceSigner,
    lamports: input.rentLamports,
    space: NONCE_ACCOUNT_SPACE,
    programAddress: SYSTEM_PROGRAM_ADDRESS,
  });

  const initIx = getInitializeNonceAccountInstruction({
    nonceAccount: nonceSigner.address,
    nonceAuthority: payerAddress,
  });

  const message = pipe(
    createTransactionMessage({ version: "legacy" }),
    (m) => setTransactionMessageFeePayerSigner(payerSigner, m),
    (m) => setTransactionMessageLifetimeUsingBlockhash(input.recentBlockhash, m),
    (m) => appendTransactionMessageInstructions([createIx, initIx], m)
  );

  const signed = await partiallySignTransactionMessageWithSigners(message);
  const txBase64 = getBase64EncodedWireTransaction(signed);

  return { txBase64, nonceAccountAddress: nonceSigner.address };
}

/**
 * Read the current nonce value (the stored blockhash) from a nonce account.
 * Throws if the account is missing or not yet initialized — the caller treats
 * that as "provision a nonce account first".
 */
export async function fetchNonceValue(nonceAccountAddress: string): Promise<string> {
  const account = await fetchNonce(solanaRpc, address(nonceAccountAddress));
  return account.data.blockhash as string;
}

/**
 * Rent-exempt minimum for an 80-byte nonce account, in lamports. This is what
 * the wallet must fund when creating its nonce account.
 */
export async function getNonceAccountRentLamports(): Promise<bigint> {
  return solanaRpc.getMinimumBalanceForRentExemption(NONCE_ACCOUNT_SPACE).send();
}

/**
 * Prepare the one-time nonce-account creation transaction for a wallet:
 * fetch the current rent-exempt minimum and a recent blockhash (the create tx
 * itself is ordinary and short-lived), then build + partially sign the
 * create+initialize tx. The returned tx still needs the wallet's signature,
 * which the device provides during the relay.
 */
export async function prepareNonceAccountCreation(
  payer: string
): Promise<CreateNonceAccountResult> {
  const rentLamports = await getNonceAccountRentLamports();
  // "confirmed", not "finalized": this is the one tx in the flow that rides a
  // perishable blockhash through the QR relay (no nonce exists yet), so don't
  // start its ~60-90s validity window 15-30s in the hole.
  const { value: recentBlockhash } = await solanaRpc
    .getLatestBlockhash({ commitment: "confirmed" })
    .send();

  return buildCreateNonceAccountTx({ payer, rentLamports, recentBlockhash });
}

/**
 * Poll until the freshly-created nonce account is readable (initialized and
 * confirmed on-chain), returning its current nonce value. Gives the create
 * transaction a moment to confirm before the first transfer reads the nonce.
 * Throws if the account never becomes readable within the attempt budget.
 */
export async function waitForNonceAccountReady(
  nonceAccountAddress: string,
  { attempts = 15, intervalMs = 1000 }: { attempts?: number; intervalMs?: number } = {}
): Promise<string> {
  let lastError: unknown;
  for (let i = 0; i < attempts; i += 1) {
    try {
      return await fetchNonceValue(nonceAccountAddress);
    } catch (err) {
      lastError = err;
      await new Promise((resolve) => setTimeout(resolve, intervalMs));
    }
  }
  throw new Error(
    `Nonce account ${nonceAccountAddress} not ready after ${attempts} attempts: ${
      lastError instanceof Error ? lastError.message : String(lastError)
    }`
  );
}
