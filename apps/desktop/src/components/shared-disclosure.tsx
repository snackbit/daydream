import type { ReactNode } from 'react';
import { ChevronDown } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible';
import { cn } from '@/lib/utils';

export interface SharedDisclosureProps {
  label: ReactNode;
  status?: ReactNode;
  children: ReactNode;
  defaultOpen?: boolean;
  className?: string;
  contentClassName?: string;
}

export function SharedDisclosure({
  label,
  status,
  children,
  defaultOpen = false,
  className,
  contentClassName,
}: SharedDisclosureProps) {
  return (
    <Collapsible
      defaultOpen={defaultOpen}
      className={cn(
        'group/disclosure rounded-panel bg-card min-w-0',
        className,
      )}
    >
      <CollapsibleTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          className="rounded-panel h-auto min-h-10 w-full justify-start gap-3 px-4 py-3 text-left"
        >
          <span className="text-control min-w-0 flex-1 font-medium">
            {label}
          </span>
          {status ? (
            <span className="text-muted-foreground text-metadata">
              {status}
            </span>
          ) : null}
          <ChevronDown
            aria-hidden="true"
            className="size-4 shrink-0 transition-transform group-data-[state=open]/disclosure:rotate-180"
          />
        </Button>
      </CollapsibleTrigger>
      <CollapsibleContent
        className={cn('text-body px-4 pb-4', contentClassName)}
      >
        {children}
      </CollapsibleContent>
    </Collapsible>
  );
}
