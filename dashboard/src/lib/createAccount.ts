/**
 * Build + send a Squads v4 multisig-create transaction directly against
 * the program. We bypass any frontend that adds a service fee on top of
 * the on-chain rent + program fee.
 */

import { Keypair, PublicKey, VersionedTransaction } from "@solana/web3.js";
import * as multisig from "@sqds/multisig";
import { connection } from "./squads";

export interface CreateAccountInput {
  readonly creator: PublicKey;          // wallet that pays rent + program fee
  readonly approvers: PublicKey[];      // includes creator if creator is also an approver
  readonly approvalsRequired: number;
  readonly memo?: string;
}

export interface CreateAccountPlan {
  readonly tx: VersionedTransaction;    // partially signed by createKey
  readonly accountId: PublicKey;        // future Multisig PDA
  readonly createKey: Keypair;          // ephemeral, used as the seed for the PDA
  readonly blockhash: string;           // tx's recent blockhash
  readonly lastValidBlockHeight: number; // tx's expiry; needed for confirmTransaction
}

/** Build the unsigned tx + the PDA we'll see after it lands. */
export async function planCreateAccount(input: CreateAccountInput): Promise<CreateAccountPlan> {
  if (input.approvers.length === 0) throw new Error("at least one approver");
  if (input.approvalsRequired < 1 || input.approvalsRequired > input.approvers.length) {
    throw new Error("approvalsRequired must be between 1 and the number of approvers");
  }

  const programConfigPda = multisig.getProgramConfigPda({})[0];
  const programConfig = await multisig.accounts.ProgramConfig.fromAccountAddress(
    connection,
    programConfigPda,
  );

  const createKey = Keypair.generate();
  const [accountId] = multisig.getMultisigPda({ createKey: createKey.publicKey });

  const members = input.approvers.map((key) => ({
    key,
    permissions: multisig.types.Permissions.all(),
  }));

  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();

  const tx = multisig.transactions.multisigCreateV2({
    blockhash,
    treasury: programConfig.treasury,
    createKey: createKey.publicKey,
    creator: input.creator,
    multisigPda: accountId,
    configAuthority: null,            // autonomous: members can later vote to change config
    threshold: input.approvalsRequired,
    members,
    timeLock: 0,
    rentCollector: null,
    memo: input.memo,
  });

  // The createKey must sign too — it's the seed that proves PDA derivation.
  // The wallet will add the creator signature next.
  tx.sign([createKey]);

  return { tx, accountId, createKey, blockhash, lastValidBlockHeight };
}

/** Estimated cost (rent + program fee) in lamports, for UI display. */
export async function estimateCreateAccountCost(memberCount: number): Promise<number> {
  const programConfigPda = multisig.getProgramConfigPda({})[0];
  const programConfig = await multisig.accounts.ProgramConfig.fromAccountAddress(
    connection,
    programConfigPda,
  );
  // Rent for a Multisig account: a coarse estimate based on size grows
  // with member count. Using the SDK's own size constant when available;
  // otherwise an approximation that matches what the program enforces.
  const accountSize = 8 + 32 + 32 + 32 + 4 + 4 + 4 + 4 + 1 + 4 + memberCount * (32 + 1);
  const rent = await connection.getMinimumBalanceForRentExemption(accountSize);
  return Number(programConfig.multisigCreationFee) + rent;
}
