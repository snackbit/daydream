import type { ReactNode } from 'react';

export function StudioSidebarSection({
  children,
  title,
}: {
  children: ReactNode;
  title: ReactNode;
}) {
  return (
    <section className="space-y-4 border-b py-6">
      <div className="text-foreground px-6 text-sm font-medium">{title}</div>
      <div className="space-y-3 px-6 text-sm">{children}</div>
    </section>
  );
}

export function MetadataRow({
  label,
  value,
}: {
  label: string;
  value: string;
}) {
  return (
    <div className="grid min-w-0 grid-cols-[5.25rem_minmax(0,1fr)] gap-3 text-xs">
      <div className="text-muted-foreground">{label}</div>
      <div className="min-w-0 break-words">{value}</div>
    </div>
  );
}
