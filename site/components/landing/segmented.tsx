"use client";

/** The sliding-pill segmented control from the hero's device switcher,
 * shared: equal-width segments, an active pill that slides between them,
 * pointer cursor, hover tint, and press feedback. */
export function Segmented<T extends string>({
  options,
  value,
  onChange,
  label,
}: {
  options: readonly (readonly [T, string])[];
  value: T;
  onChange: (value: T) => void;
  label: string;
}) {
  const idx = Math.max(
    0,
    options.findIndex(([v]) => v === value),
  );
  return (
    <div
      className="relative inline-grid rounded-sm border border-border p-1"
      style={{ gridTemplateColumns: `repeat(${options.length}, minmax(0, 1fr))` }}
      role="group"
      aria-label={label}
    >
      {/* sliding active pill */}
      <span
        aria-hidden
        className="absolute inset-y-1 left-1 rounded-[1px] bg-primary transition-transform duration-300 ease-out"
        style={{
          width: `calc((100% - 8px) / ${options.length})`,
          transform: `translateX(${idx * 100}%)`,
        }}
      />
      {options.map(([v, text]) => (
        <button
          key={v}
          aria-pressed={value === v}
          onClick={() => onChange(v)}
          className={`relative z-10 cursor-pointer rounded-[1px] px-4 py-1.5 font-mono text-[11px] uppercase tracking-[0.16em] transition-[color,transform] duration-300 active:scale-[0.96] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background ${
            value === v
              ? "text-brand"
              : "text-muted-foreground hover:bg-foreground/[0.04] hover:text-foreground"
          }`}
        >
          {text}
        </button>
      ))}
    </div>
  );
}
