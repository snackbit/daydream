import type { ReactNode } from 'react';

export function StudioSidebarSection({
  children,
  title,
}: {
  children: ReactNode;
  title: ReactNode;
}) {
  return (
    <section className="space-y-1.5 border-b pb-2">
      <div className="text-foreground/90 border-b px-2.5 py-1.5 text-[11px] font-semibold">
        {title}
      </div>
      <div className="space-y-1.5 px-2.5 text-[11px]">{children}</div>
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
    <div className="grid min-w-0 grid-cols-[4.5rem_minmax(0,1fr)] gap-1.5">
      <div className="text-muted-foreground truncate">{label}</div>
      <div className="truncate">{value}</div>
    </div>
  );
}
