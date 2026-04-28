import type { ReactNode } from "react";

export function PanelHeader({
  icon,
  title,
  subtitle
}: {
  icon: ReactNode;
  title: string;
  subtitle: string;
}) {
  return (
    <div className="flex items-center gap-2 border-b border-line px-4 py-3">
      <span className="text-accent">{icon}</span>
      <div>
        <h2 className="text-sm font-semibold">{title}</h2>
        <p className="text-xs text-muted">{subtitle}</p>
      </div>
    </div>
  );
}

export function InspectorCard({
  title,
  icon,
  children
}: {
  title: string;
  icon: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="rounded-xl border border-line bg-white p-4">
      <h3 className="mb-3 flex items-center gap-2 text-sm font-semibold">
        <span className="text-accent">{icon}</span>
        {title}
      </h3>
      <div className="space-y-2">{children}</div>
    </section>
  );
}

export function Meta({ label, value }: { label: string; value: string | null }) {
  return (
    <div>
      <div className="font-mono text-[10px] uppercase tracking-[0.14em] text-muted">{label}</div>
      <div className="mt-1 break-all font-mono text-xs text-ink">{value ?? "-"}</div>
    </div>
  );
}

export function ErrorBox({ message }: { message: string }) {
  return <div className="rounded-xl border border-red-200 bg-red-50 p-3 text-sm text-red-700">{message}</div>;
}
