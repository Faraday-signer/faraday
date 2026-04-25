import { Logo } from "./Logo";

export type Section = "accounts" | "payments" | "approvers" | "settings";

interface Props {
  current: Section;
  onChange: (s: Section) => void;
}

const NAV: { id: Section; label: string; icon: JSX.Element; ready: boolean }[] = [
  { id: "accounts",  label: "Accounts",  icon: <IconAccounts />,  ready: true },
  { id: "payments",  label: "Payments",  icon: <IconPayments />,  ready: true },
  { id: "approvers", label: "Approvers", icon: <IconApprovers />, ready: false },
  { id: "settings",  label: "Settings",  icon: <IconSettings />,  ready: false },
];

export function Sidebar({ current, onChange }: Props) {
  return (
    <aside className="w-60 shrink-0 border-r flex flex-col" style={{ borderColor: "var(--color-border)" }}>
      <div className="px-5 py-5 border-b" style={{ borderColor: "var(--color-border)" }}>
        <Logo height={20} />
      </div>
      <nav className="flex-1 py-4 px-3 space-y-1">
        {NAV.map((item) => {
          const active = item.id === current;
          return (
            <button
              key={item.id}
              onClick={() => item.ready && onChange(item.id)}
              disabled={!item.ready}
              className="w-full flex items-center gap-3 px-3 py-2 rounded-md text-sm transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
              style={{
                background: active ? "var(--color-accent-soft)" : "transparent",
                color: active ? "var(--color-accent)" : "var(--color-muted)",
              }}
              onMouseEnter={(e) => {
                if (!active && item.ready) e.currentTarget.style.background = "rgba(255,255,255,0.04)";
              }}
              onMouseLeave={(e) => {
                if (!active) e.currentTarget.style.background = "transparent";
              }}
            >
              <span className="shrink-0">{item.icon}</span>
              <span className="flex-1 text-left">{item.label}</span>
              {!item.ready && <span className="text-[10px] uppercase tracking-wider" style={{ color: "var(--color-dim)" }}>Soon</span>}
            </button>
          );
        })}
      </nav>
      <div className="px-5 py-4 border-t text-xs flex items-center gap-2" style={{ borderColor: "var(--color-border)", color: "var(--color-dim)" }}>
        <span className="w-1.5 h-1.5 rounded-full" style={{ background: "var(--color-success)" }} />
        Solana mainnet
      </div>
    </aside>
  );
}

function IconAccounts() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3" y="6" width="18" height="13" rx="2" />
      <path d="M3 10h18 M8 14h3" />
    </svg>
  );
}
function IconPayments() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M4 12h16 M14 6l6 6-6 6" />
    </svg>
  );
}
function IconApprovers() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="9" cy="8" r="3.5" />
      <path d="M3 20c0-3 2.7-5 6-5s6 2 6 5" />
      <circle cx="17" cy="9" r="2.5" />
      <path d="M15 20c0-2.5 1.7-4 3.5-4s3 1 3 3" />
    </svg>
  );
}
function IconSettings() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="3" />
      <path d="M12 2v3 M12 19v3 M2 12h3 M19 12h3 M4.9 4.9l2.1 2.1 M17 17l2.1 2.1 M4.9 19.1l2.1-2.1 M17 7l2.1-2.1" />
    </svg>
  );
}
