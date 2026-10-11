import type { ReactNode } from 'react';

export interface SettingRowProps {
  title: string;
  description?: ReactNode;
  children: ReactNode;
}
export function SettingRow({ title, description, children }: SettingRowProps) {
  return (
    <div className="flex flex-col gap-3 border-b py-5 last:border-b-0 sm:flex-row sm:items-center sm:justify-between sm:gap-8">
      <div className="min-w-0">
        <div className="text-sm font-medium">{title}</div>
        {description ? (
          <div className="text-muted-foreground mt-1 text-xs leading-5">
            {description}
          </div>
        ) : null}
      </div>
      <div className="shrink-0 sm:max-w-[55%]">{children}</div>
    </div>
  );
}
