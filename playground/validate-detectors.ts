/**
 * One-shot validation: build each scenario tx, feed it to PR #49's
 * analyzeTxRisk, and print actual vs expected.
 *
 * Run from /Users/cxalem/projects/faraday/extension:
 *   npx tsx /tmp/validate-detectors.ts
 */

import {
  ComputeBudgetProgram,
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";

import { analyzeTxRisk, type TxRiskReport } from "../extension/src/lib/tx-risk";

const RPC_URL = "https://api.devnet.solana.com";
const DUMMY_RECIPIENT = "GThUX1Atko4tqhN2NaiTazWSeFWMuiUiswQrAogXjUk7";

const connection = new Connection(RPC_URL, "confirmed");

interface Case {
  id: string;
  expectedLevel: "SAFE" | "WARNING" | "DANGER";
  expectedTitles: string[];
  build: (from: PublicKey, blockhash: string, balanceLamports: number) => Transaction;
  needsBalance: number; // minimum lamports required for the case to be meaningful
}

const CASES: Case[] = [
  {
    id: "safe-transfer",
    expectedLevel: "SAFE",
    expectedTitles: [],
    needsBalance: 0.01 * LAMPORTS_PER_SOL,
    build: (from, blockhash) =>
      new Transaction({ feePayer: from, recentBlockhash: blockhash }).add(
        SystemProgram.transfer({
          fromPubkey: from,
          toPubkey: new PublicKey(DUMMY_RECIPIENT),
          lamports: 0.001 * LAMPORTS_PER_SOL,
        })
      ),
  },
  {
    id: "sol-drain-95",
    expectedLevel: "DANGER",
    expectedTitles: ["Possible SOL Drain"],
    needsBalance: 0.5 * LAMPORTS_PER_SOL, // detector ignores below 0.1, give margin
    build: (from, blockhash, balance) => {
      const lamports = Math.floor(balance * 0.96);
      return new Transaction({ feePayer: from, recentBlockhash: blockhash }).add(
        SystemProgram.transfer({
          fromPubkey: from,
          toPubkey: new PublicKey(DUMMY_RECIPIENT),
          lamports,
        })
      );
    },
  },
  {
    id: "oversized-priority-fee",
    expectedLevel: "WARNING",
    expectedTitles: ["Oversized Priority Fee"],
    needsBalance: 0.01 * LAMPORTS_PER_SOL,
    build: (from, blockhash) =>
      new Transaction({ feePayer: from, recentBlockhash: blockhash })
        .add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }))
        .add(ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 50_000_000 }))
        .add(
          SystemProgram.transfer({
            fromPubkey: from,
            toPubkey: new PublicKey(DUMMY_RECIPIENT),
            lamports: 1_000,
          })
        ),
  },
  {
    id: "simulation-failure",
    expectedLevel: "DANGER",
    expectedTitles: ["Transaction Would Fail"],
    needsBalance: 0,
    build: (from, blockhash) =>
      new Transaction({ feePayer: from, recentBlockhash: blockhash }).add(
        SystemProgram.transfer({
          fromPubkey: from,
          toPubkey: new PublicKey(DUMMY_RECIPIENT),
          lamports: 99_999 * LAMPORTS_PER_SOL,
        })
      ),
  },
];

function summarize(report: TxRiskReport): string {
  return (
    `level=${report.level}, simFailed=${report.simulationFailed}, ` +
    `warnings=[${report.warnings.map((w) => w.title).join(", ")}]`
  );
}

function check(report: TxRiskReport, expected: Case): { pass: boolean; reason?: string } {
  if (report.level !== expected.expectedLevel) {
    return { pass: false, reason: `level mismatch: got ${report.level}, expected ${expected.expectedLevel}` };
  }
  for (const title of expected.expectedTitles) {
    if (!report.warnings.some((w) => w.title === title)) {
      return { pass: false, reason: `missing expected warning: "${title}"` };
    }
  }
  return { pass: true };
}

async function main() {
  console.log("─── PR #49 detector validation ───\n");

  const kp = Keypair.generate();
  console.log(`Test keypair: ${kp.publicKey.toBase58()}\n`);

  // Try to fund. Devnet airdrop is heavily rate-limited; if it fails, we run
  // only the simulation-failure case (which works on a fresh address).
  let funded = false;
  console.log("Requesting devnet airdrop (1 SOL)…");
  try {
    const sig = await connection.requestAirdrop(kp.publicKey, 1 * LAMPORTS_PER_SOL);
    await connection.confirmTransaction(sig, "confirmed");
    funded = true;
    console.log("  ✓ Funded.\n");
  } catch (err) {
    console.log(`  ✗ Airdrop failed: ${err instanceof Error ? err.message : err}`);
    console.log("  Will skip cases that need balance.\n");
  }

  const balance = funded ? await connection.getBalance(kp.publicKey) : 0;
  if (funded) {
    console.log(`Balance: ${(balance / LAMPORTS_PER_SOL).toFixed(4)} SOL\n`);
  }

  const { blockhash } = await connection.getLatestBlockhash("confirmed");

  let pass = 0;
  let fail = 0;
  let skip = 0;

  for (const c of CASES) {
    if (c.needsBalance > balance) {
      console.log(`[SKIP] ${c.id} (needs ${(c.needsBalance / LAMPORTS_PER_SOL).toFixed(4)} SOL)`);
      skip++;
      continue;
    }

    const tx = c.build(kp.publicKey, blockhash, balance);
    const bytes = tx.serialize({ requireAllSignatures: false, verifySignatures: false });
    const txBase64 = Buffer.from(bytes).toString("base64");

    let report: TxRiskReport;
    try {
      report = await analyzeTxRisk(txBase64, RPC_URL, kp.publicKey.toBase58());
    } catch (err) {
      console.log(`[ERR ] ${c.id}: ${err instanceof Error ? err.message : err}`);
      fail++;
      continue;
    }

    const verdict = check(report, c);
    const tag = verdict.pass ? "[PASS]" : "[FAIL]";
    console.log(`${tag} ${c.id}: ${summarize(report)}`);
    if (verdict.pass) pass++;
    else {
      console.log(`        reason: ${verdict.reason}`);
      fail++;
    }
  }

  console.log(`\n─── ${pass} pass · ${fail} fail · ${skip} skip ───`);
  if (fail > 0) process.exit(1);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
