import { useId, type ReactNode } from 'react';

export interface InspectorSectionProps {
  children: ReactNode;
  title: string;
}

export function InspectorSection({ children, title }: InspectorSectionProps) {
  const headingId = useId();

  return (
    <section aria-labelledby={headingId} className="border-b py-6">
      <h3 className="px-6 text-sm font-medium" id={headingId}>
        {title}
      </h3>
      <div className="mt-4 space-y-3 px-6 text-sm">{children}</div>
    </section>
  );
}

export interface InspectorFieldProps {
  label: string;
  value: ReactNode;
}

export function InspectorField({ label, value }: InspectorFieldProps) {
  return (
    <div className="grid min-w-0 grid-cols-[5.25rem_minmax(0,1fr)] gap-2 leading-4">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="min-w-0 break-words">{value}</dd>
    </div>
  );
}
