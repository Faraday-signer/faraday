/**
 * Risk-detector test scenarios.
 *
 * Each scenario builds an unsigned `Transaction` whose shape is designed to
 * trigger one specific detector in `extension/src/lib/tx-risk.ts` (see PR #49).
 * The popup that opens during signing should display the expected level +
 * warning title — that's the manual validation.
 *
 * Devnet only. Recipient defaults to a fixed dummy address so the test isn't
 * coupled to a specific wallet. Builders that need balance preconditions
 * throw a friendly message rather than producing a tx that won't trigger
 * what we want (so users get a clear "airdrop first" instead of a
 * confusing SAFE result).
 *
 * Detector thresholds (from tx-risk.ts):
 *   - DRAIN_WIPE_RATIO          0.95
 *   - HIGH_VALUE_SOL_THRESHOLD  10 SOL
 *   - OVERSIZED_PRIORITY_FEE    0.05 SOL  (microLamports * unitsConsumed / 1e6)
 *   - SOL drain ignores wallets with < 0.1 SOL pre-tx
 */

import {
  ComputeBudgetProgram,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";

import { connection } from "./wallet";

/** Dummy devnet address used as the destination for control transfers. */
const DUMMY_RECIPIENT = "GThUX1Atko4tqhN2NaiTazWSeFWMuiUiswQrAogXjUk7";

export type ExpectedLevel = "SAFE" | "WARNING" | "DANGER";

export interface BuildContext {
  fromPubkey: PublicKey;
  blockhash: string;
}

export interface BuildResult {
  tx: Transaction;
  /** Optional human-friendly note shown in the run log alongside the result. */
  note?: string;
}

export interface Scenario {
  id: string;
  label: string;
  description: string;
  expectedLevel: ExpectedLevel;
  /** Warning titles the popup should display. Empty array = no warnings (SAFE). */
  expectedTitles: string[];
  build: (ctx: BuildContext) => Promise<BuildResult>;
}

export const SCENARIOS: Scenario[] = [
  {
    id: "safe-transfer",
    label: "Control · safe transfer",
    description:
      "Send 0.001 SOL to a dummy address. Should be SAFE with no warnings — sanity-check that the harness wiring works.",
    expectedLevel: "SAFE",
    expectedTitles: [],
    build: async ({ fromPubkey, blockhash }) => {
      const tx = new Transaction({
        feePayer: fromPubkey,
        recentBlockhash: blockhash,
      }).add(
        SystemProgram.transfer({
          fromPubkey,
          toPubkey: new PublicKey(DUMMY_RECIPIENT),
          lamports: Math.floor(0.001 * LAMPORTS_PER_SOL),
        })
      );
      return { tx };
    },
  },

  {
    id: "sol-drain-95",
    label: "SOL drain · 96% of balance",
    description:
      "Send 96% of your devnet SOL balance. Should be DANGER — “Possible SOL Drain”. The detector ignores wallets below 0.1 SOL, so airdrop first if you're empty.",
    expectedLevel: "DANGER",
    expectedTitles: ["Possible SOL Drain"],
    build: async ({ fromPubkey, blockhash }) => {
      const balance = await connection.getBalance(fromPubkey);
      if (balance < 0.1 * LAMPORTS_PER_SOL) {
        throw new Error(
          `Need ≥0.1 SOL on devnet for this detector. You have ${(
            balance / LAMPORTS_PER_SOL
          ).toFixed(4)} SOL. Run "Airdrop 1 SOL" first.`
        );
      }
      const lamports = Math.floor(balance * 0.96);
      const tx = new Transaction({
        feePayer: fromPubkey,
        recentBlockhash: blockhash,
      }).add(
        SystemProgram.transfer({
          fromPubkey,
          toPubkey: new PublicKey(DUMMY_RECIPIENT),
          lamports,
        })
      );
      return {
        tx,
        note: `Drains ${(lamports / LAMPORTS_PER_SOL).toFixed(4)} of ${(
          balance / LAMPORTS_PER_SOL
        ).toFixed(4)} SOL (~96%).`,
      };
    },
  },

  {
    id: "significant-sol-outflow",
    label: "Significant SOL outflow · 70% of balance",
    description:
      "Sends 70% of your devnet SOL — falls inside the new 50–95% tier. Should be WARNING / Quality (not the critical drain). Tests the graduated drain detection.",
    expectedLevel: "WARNING",
    expectedTitles: ["Significant SOL outflow"],
    build: async ({ fromPubkey, blockhash }) => {
      const balance = await connection.getBalance(fromPubkey);
      if (balance < 0.1 * LAMPORTS_PER_SOL) {
        throw new Error(
          `Need ≥0.1 SOL for the drain detectors to fire. You have ${(
            balance / LAMPORTS_PER_SOL
          ).toFixed(4)} SOL. Run "Airdrop 1 SOL" first.`,
        );
      }
      const lamports = Math.floor(balance * 0.7);
      const tx = new Transaction({
        feePayer: fromPubkey,
        recentBlockhash: blockhash,
      }).add(
        SystemProgram.transfer({
          fromPubkey,
          toPubkey: new PublicKey(DUMMY_RECIPIENT),
          lamports,
        }),
      );
      return {
        tx,
        note: `Moves ${(lamports / LAMPORTS_PER_SOL).toFixed(4)} of ${(
          balance / LAMPORTS_PER_SOL
        ).toFixed(4)} SOL (~70%) — between the safe and drain tiers.`,
      };
    },
  },

  {
    id: "oversized-priority-fee",
    label: "Oversized priority fee",
    description:
      "Adds setComputeUnitLimit(1.4M) + setComputeUnitPrice(50M µ-lamports) on top of a tiny SOL transfer. Implied priority fee ≫ 0.05 SOL → WARNING “Oversized Priority Fee”.",
    expectedLevel: "WARNING",
    expectedTitles: ["Oversized Priority Fee"],
    build: async ({ fromPubkey, blockhash }) => {
      const tx = new Transaction({
        feePayer: fromPubkey,
        recentBlockhash: blockhash,
      })
        .add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }))
        .add(ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 50_000_000 }))
        .add(
          SystemProgram.transfer({
            fromPubkey,
            toPubkey: new PublicKey(DUMMY_RECIPIENT),
            lamports: 1_000,
          })
        );
      return {
        tx,
        note:
          "If the simulation can't afford the implied priority fee it'll also fail (DANGER “Transaction Would Fail”) — either way the popup should warn before signing.",
      };
    },
  },

  {
    id: "simulation-failure",
    label: "Insufficient funds · simulation fails",
    description:
      "Tries to send 99,999 SOL. The on-chain simulation returns an error → DANGER “Transaction Would Fail”. Tests the err-path classifier (NOT graceful WARNING — that path is reserved for RPC unreachable).",
    expectedLevel: "DANGER",
    expectedTitles: ["Transaction Would Fail"],
    build: async ({ fromPubkey, blockhash }) => {
      const tx = new Transaction({
        feePayer: fromPubkey,
        recentBlockhash: blockhash,
      }).add(
        SystemProgram.transfer({
          fromPubkey,
          toPubkey: new PublicKey(DUMMY_RECIPIENT),
          lamports: 99_999 * LAMPORTS_PER_SOL,
        })
      );
      return { tx };
    },
  },

  {
    id: "high-value-sol",
    label: "High-value transfer · 10 SOL",
    description:
      "Sends exactly 10 SOL. Triggers the absolute-value detector → WARNING “High Value Transfer”. Needs ≥10 SOL on devnet — devnet airdrop gives 1 SOL/request, so multiple airdrops required.",
    expectedLevel: "WARNING",
    expectedTitles: ["High Value Transfer"],
    build: async ({ fromPubkey, blockhash }) => {
      const balance = await connection.getBalance(fromPubkey);
      const required = 10 * LAMPORTS_PER_SOL + 1_000_000; // small fee buffer
      if (balance < required) {
        throw new Error(
          `Need ≥10 SOL on devnet. You have ${(balance / LAMPORTS_PER_SOL).toFixed(
            4
          )} SOL. Devnet airdrop gives 1 SOL per request — repeat 10+ times, or use a faucet site.`
        );
      }
      const tx = new Transaction({
        feePayer: fromPubkey,
        recentBlockhash: blockhash,
      }).add(
        SystemProgram.transfer({
          fromPubkey,
          toPubkey: new PublicKey(DUMMY_RECIPIENT),
          lamports: 10 * LAMPORTS_PER_SOL,
        })
      );
      return { tx };
    },
  },
];

export function findScenario(id: string): Scenario | undefined {
  return SCENARIOS.find((s) => s.id === id);
}
