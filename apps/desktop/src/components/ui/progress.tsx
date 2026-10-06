import * as React from 'react';
import { Progress as ProgressPrimitive } from 'radix-ui';
import { cn } from '@/lib/utils';

function Progress({
  className,
  value,
  ...props
}: React.ComponentProps<typeof ProgressPrimitive.Root>) {
  return (
    <ProgressPrimitive.Root
      data-slot="progress"
      value={value}
      className={cn(
        'bg-muted relative flex h-1 w-full items-center overflow-x-hidden rounded-md',
        className,
      )}
      {...props}
    >
      <ProgressPrimitive.Indicator
        data-slot="progress-indicator"
        className="bg-primary size-full flex-1 transition-all data-[state=indeterminate]:w-1/3 data-[state=indeterminate]:animate-pulse"
        style={{
          transform: value == null ? 'none' : `translateX(-${100 - value}%)`,
        }}
      />
    </ProgressPrimitive.Root>
  );
}

export { Progress };
