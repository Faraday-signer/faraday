import { useState } from "react";
import { copy, shortId } from "../lib/format";
import type { DiscoveredWallet } from "../lib/wallet";

interface Props {
  wallets: DiscoveredWallet[];
  pubkey: string | null;
  walletName: string | null;
  onConnect: (w: DiscoveredWallet) => void;
  onDisconnect: () => void;
}

export function Topbar({ wallets, pubkey, walletName, onConnect, onDisconnect }: Props) {
  const [open, setOpen] = useState(false);

  return (
    <header
      className="h-14 flex items-center justify-between px-6 border-b"
      style={{ borderColor: "var(--color-border)", background: "var(--color-bg)" }}
    >
      <div className="text-xs uppercase tracking-[0.18em]" style={{ color: "var(--color-dim)" }}>
        Treasury
      </div>

      {pubkey ? (
        <ConnectedMenu
          name={walletName ?? "Wallet"}
          pubkey={pubkey}
          onDisconnect={onDisconnect}
        />
      ) : wallets.length === 0 ? (
        <span className="text-xs" style={{ color: "var(--color-muted)" }}>
          Install a Solana wallet to sign in
        </span>
      ) : (
        <div className="relative">
          <button
            onClick={() => setOpen((o) => !o)}
            onBlur={() => setTimeout(() => setOpen(false), 120)}
            className="px-3.5 py-1.5 rounded-md text-sm font-medium glow-accent"
            style={{
              background: "var(--color-accent)",
              color: "#001721",
            }}
          >
            Sign in
          </button>
          {open && (
            <div
              className="absolute right-0 mt-2 w-56 rounded-md border py-1 shadow-xl z-10"
              style={{ background: "var(--color-elevated)", borderColor: "var(--color-border-strong)" }}
            >
              <div className="px-3 py-2 text-[11px] uppercase tracking-wider" style={{ color: "var(--color-dim)" }}>
                Choose a wallet
              </div>
              {wallets.map((w) => (
                <button
                  key={w.name}
                  onMouseDown={() => { onConnect(w); setOpen(false); }}
                  className="w-full flex items-center gap-3 px-3 py-2 text-sm hover:bg-white/5"
                  style={{ color: "var(--color-fg)" }}
                >
                  {w.icon ? (
                    <img src={w.icon} alt="" className="w-5 h-5 rounded" />
                  ) : (
                    <div className="w-5 h-5 rounded bg-white/10" />
                  )}
                  <span>{w.name}</span>
                </button>
              ))}
            </div>
          )}
        </div>
      )}
    </header>
  );
}

function ConnectedMenu({ name, pubkey, onDisconnect }: { name: string; pubkey: string; onDisconnect: () => void }) {
  const [open, setOpen] = useState(false);
  const [copied, setCopied] = useState(false);

  async function onCopy() {
    if (await copy(pubkey)) {
      setCopied(true);
      setTimeout(() => setCopied(false), 1200);
    }
  }

  return (
    <div className="relative">
      <button
        onClick={() => setOpen((o) => !o)}
        onBlur={() => setTimeout(() => setOpen(false), 120)}
        className="flex items-center gap-2.5 px-3 py-1.5 rounded-md border text-sm hover:bg-white/[0.03]"
        style={{ borderColor: "var(--color-border)" }}
      >
        <Avatar seed={pubkey} />
        <div className="text-left leading-tight">
          <div className="text-[11px] uppercase tracking-wider" style={{ color: "var(--color-dim)" }}>{name}</div>
          <div className="font-mono text-xs" style={{ color: "var(--color-fg)" }}>{shortId(pubkey)}</div>
        </div>
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" style={{ color: "var(--color-dim)" }}>
          <path d="M6 9l6 6 6-6" />
        </svg>
      </button>

      {open && (
        <div
          className="absolute right-0 mt-2 w-72 rounded-md border py-1.5 shadow-xl z-10"
          style={{ background: "var(--color-elevated)", borderColor: "var(--color-border-strong)" }}
        >
          <div className="px-3 py-2.5 border-b" style={{ borderColor: "var(--color-border)" }}>
            <div className="text-[11px] uppercase tracking-wider mb-1" style={{ color: "var(--color-dim)" }}>Wallet address</div>
            <div className="font-mono text-xs break-all" style={{ color: "var(--color-fg)" }}>{pubkey}</div>
          </div>
          <button
            onMouseDown={onCopy}
            className="w-full text-left px-3 py-2 text-sm hover:bg-white/5"
            style={{ color: "var(--color-fg)" }}
          >
            {copied ? "Copied" : "Copy address"}
          </button>
          <button
            onMouseDown={onDisconnect}
            className="w-full text-left px-3 py-2 text-sm hover:bg-white/5"
            style={{ color: "var(--color-danger)" }}
          >
            Disconnect
          </button>
        </div>
      )}
    </div>
  );
}

function Avatar({ seed }: { seed: string }) {
  // Deterministic 3-color stripe from the address — recognisable at a glance
  // without needing a third-party identicon library.
  const hash = Array.from(seed).reduce((acc, ch) => (acc * 31 + ch.charCodeAt(0)) >>> 0, 5381);
  const h = hash % 360;
  return (
    <div
      className="w-7 h-7 rounded-full"
      style={{
        background: `conic-gradient(from ${hash % 360}deg, hsl(${h} 80% 60%), hsl(${(h + 60) % 360} 80% 50%), hsl(${(h + 180) % 360} 80% 60%))`,
      }}
    />
  );
}
