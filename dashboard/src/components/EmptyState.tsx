import type { ReactNode } from "react";

export function EmptyState({
  title,
  description,
  cta,
  icon,
}: {
  title: string;
  description: string;
  cta?: ReactNode;
  icon?: ReactNode;
}) {
  return (
    <div
      className="rounded-lg border p-12 text-center"
      style={{ background: "var(--color-surface)", borderColor: "var(--color-border)" }}
    >
      {icon && (
        <div className="mx-auto mb-4 w-12 h-12 rounded-full flex items-center justify-center"
             style={{ background: "var(--color-accent-soft)", color: "var(--color-accent)" }}>
          {icon}
        </div>
      )}
      <h3 className="text-base font-medium mb-1" style={{ color: "var(--color-fg)" }}>{title}</h3>
      <p className="text-sm mb-5 max-w-sm mx-auto" style={{ color: "var(--color-muted)" }}>{description}</p>
      {cta}
    </div>
  );
}
