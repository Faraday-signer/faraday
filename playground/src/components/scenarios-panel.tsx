import { useState } from "react";
import { PublicKey } from "@solana/web3.js";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { findScenario, SCENARIOS, type ExpectedLevel } from "@/lib/scenarios";
import { connection, explorerTxUrl, signAndSendBuiltTx, type StandardAccount, type SupportedWallet } from "@/lib/wallet";

interface ScenariosPanelProps {
  wallet: SupportedWallet | null;
  account: { address: string } | null;
  busy: boolean;
  setBusy: (next: boolean) => void;
  log: (message: string, level?: "info" | "error" | "success") => void;
}

const LEVEL_BADGE_CLASS: Record<ExpectedLevel, string> = {
  SAFE: "border-emerald-500/40 bg-emerald-500/10 text-emerald-400",
  WARNING: "border-amber-500/40 bg-amber-500/10 text-amber-400",
  DANGER: "border-red-500/40 bg-red-500/10 text-red-400",
};

export function ScenariosPanel({
  wallet,
  account,
  busy,
  setBusy,
  log,
}: ScenariosPanelProps) {
  const [scenarioId, setScenarioId] = useState<string>(SCENARIOS[0]?.id ?? "");
  const scenario = findScenario(scenarioId);

  const connected = Boolean(wallet && account);

  async function runScenario() {
    if (!wallet || !account || !scenario) {
      log("Connect a wallet and pick a scenario first.", "error");
      return;
    }
    setBusy(true);
    try {
      log(
        `▶ Running scenario "${scenario.label}". Expected: ${scenario.expectedLevel}` +
          (scenario.expectedTitles.length > 0
            ? ` · "${scenario.expectedTitles.join("\", \"")}"`
            : "") +
          ".",
        "info"
      );

      const fromPubkey = new PublicKey(account.address);
      const { blockhash } = await connection.getLatestBlockhash("confirmed");
      const built = await scenario.build({ fromPubkey, blockhash });

      if (built.note) log(`ℹ ${built.note}`, "info");

      log(
        "Watch the Faraday popup that opens for the signing flow — it should display the expected risk level/warning before the QR.",
        "info"
      );

      const standardAccount: StandardAccount | undefined = wallet.accounts[0];
      if (!standardAccount) {
        throw new Error("Wallet has no accounts.");
      }

      const { signature } = await signAndSendBuiltTx({
        wallet,
        account: standardAccount,
        tx: built.tx,
      });

      log(`✔ Broadcast — ${explorerTxUrl(signature)}`, "success");
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      // Most scenarios are EXPECTED to cancel/fail — surface as info, not error,
      // unless the message looks like a real bug.
      const looksLikeUserCancel =
        /user|cancel|reject|denied/i.test(message) || /sign(ing)? cancel/i.test(message);
      log(
        `${looksLikeUserCancel ? "✖ Canceled / rejected" : "✖ Run ended"}: ${message}`,
        looksLikeUserCancel ? "info" : "error"
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Risk-detector scenarios</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        <p className="text-sm text-muted-foreground">
          Each scenario builds an unsigned transaction crafted to trigger one
          detector in the extension's <code className="font-mono text-xs">tx-risk.ts</code>.
          The Faraday popup should display the expected level/warning <em>before</em> the QR.
          You can cancel after seeing the warning — broadcast isn't required.
        </p>

        <div className="space-y-2">
          <Label htmlFor="scenario">Scenario</Label>
          <Select value={scenarioId} onValueChange={setScenarioId} disabled={busy || !connected}>
            <SelectTrigger id="scenario">
              <SelectValue placeholder="Pick a scenario" />
            </SelectTrigger>
            <SelectContent>
              {SCENARIOS.map((s) => (
                <SelectItem key={s.id} value={s.id}>
                  {s.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {scenario ? (
          <div className="space-y-3 rounded-md border border-border bg-background px-3 py-3">
            <div className="flex flex-wrap items-center gap-2">
              <span
                className={`inline-flex items-center gap-1 rounded-full border px-2 py-0.5 font-mono text-[10px] uppercase tracking-[0.2em] ${
                  LEVEL_BADGE_CLASS[scenario.expectedLevel]
                }`}
              >
                Expected · {scenario.expectedLevel}
              </span>
              {scenario.expectedTitles.map((title) => (
                <span
                  key={title}
                  className="rounded-full border border-border bg-muted px-2 py-0.5 font-mono text-[10px] tracking-wide text-foreground"
                >
                  {title}
                </span>
              ))}
            </div>
            <p className="text-xs leading-relaxed text-muted-foreground">{scenario.description}</p>
          </div>
        ) : null}

        <div className="flex flex-wrap gap-2 pt-1">
          <Button onClick={runScenario} disabled={busy || !connected || !scenario}>
            Run scenario
          </Button>
        </div>

        {!connected ? (
          <p className="text-xs text-muted-foreground">Connect a wallet first.</p>
        ) : null}
      </CardContent>
    </Card>
  );
}
