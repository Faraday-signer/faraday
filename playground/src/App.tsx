import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ExternalLink, RefreshCw } from "lucide-react";

import { FaradayLogo } from "@/components/brand-logo";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue
} from "@/components/ui/select";
import {
  connectWallet,
  detectWallets,
  disconnectWallet,
  explorerTxUrl,
  requestAirdrop,
  shortAddress,
  signAndSendTransfer,
  walletsApi,
  type SupportedWallet
} from "@/lib/wallet";

interface LogEntry {
  id: string;
  time: string;
  message: string;
  level: "info" | "error" | "success";
}

interface ConnectedAccount {
  address: string;
}

export default function App() {
  const [wallets, setWallets] = useState<SupportedWallet[]>([]);
  const [selectedWalletName, setSelectedWalletName] = useState<string>("");
  const [activeWallet, setActiveWallet] = useState<SupportedWallet | null>(null);
  const [account, setAccount] = useState<ConnectedAccount | null>(null);
  const [recipient, setRecipient] = useState("");
  const [amount, setAmount] = useState("0.001");
  const [busy, setBusy] = useState(false);
  const [logs, setLogs] = useState<LogEntry[]>([]);

  const logRef = useRef<HTMLDivElement | null>(null);

  const log = useCallback(
    (message: string, level: LogEntry["level"] = "info") => {
      setLogs((prev) => [
        {
          id: `${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
          time: new Date().toLocaleTimeString(),
          message,
          level
        },
        ...prev
      ]);
    },
    []
  );

  const refreshWallets = useCallback(() => {
    const found = detectWallets();
    setWallets(found);
    if (found.length === 0) {
      log("No Wallet Standard wallets detected on this page.", "error");
      return;
    }
    if (!selectedWalletName || !found.some((w) => w.name === selectedWalletName)) {
      setSelectedWalletName(found[0].name);
    }
    log(`Detected wallets: ${found.map((w) => w.name).join(", ")}`);
  }, [log, selectedWalletName]);

  useEffect(() => {
    refreshWallets();
    const offRegister = walletsApi.on("register", refreshWallets);
    const offUnregister = walletsApi.on("unregister", refreshWallets);
    log("Playground loaded.");
    return () => {
      offRegister();
      offUnregister();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const selectedWallet = useMemo(
    () => wallets.find((w) => w.name === selectedWalletName) ?? null,
    [wallets, selectedWalletName]
  );

  const connected = Boolean(activeWallet && account);

  async function withBusy(task: () => Promise<void>, errorLabel: string) {
    setBusy(true);
    try {
      await task();
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      log(`${errorLabel}: ${message}`, "error");
    } finally {
      setBusy(false);
    }
  }

  async function handleConnect() {
    if (!selectedWallet) {
      log("Pick a wallet first.", "error");
      return;
    }
    await withBusy(async () => {
      const acc = await connectWallet(selectedWallet);
      setActiveWallet(selectedWallet);
      setAccount({ address: acc.address });
      log(`Connected ${selectedWallet.name} (${shortAddress(acc.address)})`, "success");
    }, "Connect failed");
  }

  async function handleDisconnect() {
    if (!activeWallet) return;
    await withBusy(async () => {
      await disconnectWallet(activeWallet);
      setActiveWallet(null);
      setAccount(null);
      log("Disconnected wallet.");
    }, "Disconnect failed");
  }

  async function handleAirdrop() {
    if (!account) {
      log("Connect wallet first.", "error");
      return;
    }
    await withBusy(async () => {
      log(`Requesting devnet airdrop for ${account.address}…`);
      const signature = await requestAirdrop(account.address);
      log(`Airdrop confirmed — ${explorerTxUrl(signature)}`, "success");
    }, "Airdrop failed");
  }

  async function handleSignAndSend() {
    if (!activeWallet || !account) {
      log("Connect wallet first.", "error");
      return;
    }
    const amountSol = Number(amount);
    await withBusy(async () => {
      log(
        `Requesting signature for ${amountSol} SOL → ${shortAddress(recipient || account.address)}. ` +
          "Scan the unsigned QR in the Faraday window."
      );
      const standardAccount = activeWallet.accounts[0];
      if (!standardAccount) {
        throw new Error("Wallet has no accounts.");
      }
      const { signature } = await signAndSendTransfer({
        wallet: activeWallet,
        account: standardAccount,
        recipient: recipient.trim(),
        amountSol
      });
      log(`Broadcast confirmed — ${explorerTxUrl(signature)}`, "success");
    }, "Sign + send failed");
  }

  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="border-b border-border">
        <div className="mx-auto flex max-w-5xl items-center justify-between px-6 py-5">
          <FaradayLogo height={26} color="var(--brand)" title="Faraday" />
          <span className="text-xs uppercase tracking-[0.2em] text-muted-foreground">
            Devnet · Playground
          </span>
        </div>
      </header>

      <main className="mx-auto flex max-w-5xl flex-col gap-6 px-6 py-10">
        <div className="space-y-2">
          <h1 className="text-2xl font-semibold tracking-tight">Test the air-gapped sign flow</h1>
          <p className="max-w-2xl text-sm text-muted-foreground">
            Exercises Wallet Standard <code className="font-mono text-xs">connect</code> and{" "}
            <code className="font-mono text-xs">signTransaction</code> against the Faraday extension on
            Solana devnet. The sign request pops the Faraday window; scan the QR on your device,
            then scan the signed response back.
          </p>
        </div>

        <div className="grid gap-6 md:grid-cols-2">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center justify-between">
                Wallet
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={refreshWallets}
                  disabled={busy}
                  aria-label="Refresh wallet list"
                >
                  <RefreshCw className="h-4 w-4" />
                </Button>
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="wallet">Detected</Label>
                <Select
                  value={selectedWalletName}
                  onValueChange={setSelectedWalletName}
                  disabled={wallets.length === 0 || connected}
                >
                  <SelectTrigger id="wallet">
                    <SelectValue
                      placeholder={wallets.length === 0 ? "No wallets found" : "Pick a wallet"}
                    />
                  </SelectTrigger>
                  <SelectContent>
                    {wallets.map((w) => (
                      <SelectItem key={w.name} value={w.name}>
                        {w.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              <div className="flex flex-wrap gap-2">
                {connected ? (
                  <Button variant="outline" onClick={handleDisconnect} disabled={busy}>
                    Disconnect
                  </Button>
                ) : (
                  <Button onClick={handleConnect} disabled={busy || !selectedWallet}>
                    Connect
                  </Button>
                )}
              </div>

              <div className="rounded-md border border-border bg-background px-3 py-2 font-mono text-xs text-muted-foreground">
                {connected && account ? (
                  <span className="text-foreground">
                    {activeWallet?.name} · {shortAddress(account.address)}
                  </span>
                ) : (
                  "Not connected"
                )}
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Devnet actions</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="recipient">Recipient address</Label>
                <Input
                  id="recipient"
                  value={recipient}
                  onChange={(event) => setRecipient(event.target.value)}
                  placeholder="Defaults to your own address"
                  spellCheck={false}
                  className="font-mono text-xs"
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="amount">Amount (SOL)</Label>
                <Input
                  id="amount"
                  type="number"
                  min="0"
                  step="0.000001"
                  value={amount}
                  onChange={(event) => setAmount(event.target.value)}
                  className="font-mono text-xs"
                />
              </div>

              <div className="flex flex-wrap gap-2 pt-1">
                <Button variant="outline" onClick={handleAirdrop} disabled={busy || !connected}>
                  Airdrop 1 SOL
                </Button>
                <Button onClick={handleSignAndSend} disabled={busy || !connected}>
                  Sign + send transfer
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>

        <Card>
          <CardHeader>
            <CardTitle>Logs</CardTitle>
          </CardHeader>
          <CardContent>
            <div
              ref={logRef}
              className="max-h-80 overflow-y-auto rounded-md border border-border bg-background p-3 font-mono text-xs leading-relaxed"
            >
              {logs.length === 0 ? (
                <p className="text-muted-foreground">No events yet.</p>
              ) : (
                <ul className="space-y-1">
                  {logs.map((entry) => (
                    <li
                      key={entry.id}
                      className={
                        entry.level === "error"
                          ? "text-destructive"
                          : entry.level === "success"
                            ? "text-primary"
                            : "text-muted-foreground"
                      }
                    >
                      <span className="text-muted-foreground">[{entry.time}]</span>{" "}
                      <LinkifiedMessage message={entry.message} />
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </CardContent>
        </Card>
      </main>
    </div>
  );
}

function LinkifiedMessage({ message }: { message: string }) {
  const parts = message.split(/(https?:\/\/\S+)/g);
  return (
    <>
      {parts.map((part, index) =>
        /^https?:\/\//.test(part) ? (
          <a
            key={index}
            href={part}
            target="_blank"
            rel="noreferrer"
            className="inline-flex items-center gap-1 underline underline-offset-2 hover:text-primary"
          >
            {part}
            <ExternalLink className="h-3 w-3" />
          </a>
        ) : (
          <span key={index}>{part}</span>
        )
      )}
    </>
  );
}
