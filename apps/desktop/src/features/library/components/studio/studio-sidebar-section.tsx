import type { ReactNode } from 'react';

export function StudioSidebarSection({
  children,
  title,
}: {
  children: ReactNode;
  title: ReactNode;
}) {
  return (
    <section className="space-y-2 border-b">
      <div className="text-muted-foreground bg-background border-b p-3 font-medium">
        {title}
      </div>
      <div className="space-y-2 p-3">{children}</div>
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
    <div className="grid grid-cols-[5rem_1fr] gap-2">
      <div className="text-muted-foreground">{label}</div>
      <div className="truncate">{value}</div>
    </div>
  );
}
