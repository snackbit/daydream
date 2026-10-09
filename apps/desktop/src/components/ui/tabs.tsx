import * as React from 'react';
import { Tabs as TabsPrimitive } from 'radix-ui';
import { cn } from '@/lib/utils';

function Tabs({
  className,
  ...props
}: React.ComponentProps<typeof TabsPrimitive.Root>) {
  return (
    <TabsPrimitive.Root
      data-slot="tabs"
      className={cn('flex flex-col gap-4', className)}
      {...props}
    />
  );
}

function TabsList({
  className,
  variant = 'pill',
  ...props
}: React.ComponentProps<typeof TabsPrimitive.List> & {
  variant?: 'pill' | 'underline';
}) {
  return (
    <TabsPrimitive.List
      data-slot="tabs-list"
      className={cn(
        'bg-card text-muted-foreground inline-flex h-11 w-fit max-w-full items-center justify-start gap-1 overflow-x-auto rounded-full p-1',
        variant === 'underline' &&
          '[&_[data-slot=tabs-trigger][data-state=active]]:border-b-primary h-auto justify-start rounded-none bg-transparent p-0 [&_[data-slot=tabs-trigger]]:h-9 [&_[data-slot=tabs-trigger]]:rounded-none [&_[data-slot=tabs-trigger]]:border-0 [&_[data-slot=tabs-trigger]]:border-b-[3px] [&_[data-slot=tabs-trigger]]:border-b-transparent [&_[data-slot=tabs-trigger]]:bg-transparent [&_[data-slot=tabs-trigger]]:shadow-none',
        className,
      )}
      {...props}
    />
  );
}

function TabsTrigger({
  className,
  ...props
}: React.ComponentProps<typeof TabsPrimitive.Trigger>) {
  return (
    <TabsPrimitive.Trigger
      data-slot="tabs-trigger"
      className={cn(
        'focus-visible:border-ring focus-visible:ring-ring focus-visible:outline-ring data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:border-input text-control inline-flex h-9 shrink-0 items-center justify-center gap-2 rounded-full border border-transparent px-4 font-medium whitespace-nowrap transition-colors focus-visible:ring-2 focus-visible:outline-1 disabled:pointer-events-none disabled:opacity-50',
        className,
      )}
      {...props}
    />
  );
}

function TabsContent({
  className,
  ...props
}: React.ComponentProps<typeof TabsPrimitive.Content>) {
  return (
    <TabsPrimitive.Content
      data-slot="tabs-content"
      className={cn('outline-none', className)}
      {...props}
    />
  );
}

export { Tabs, TabsContent, TabsList, TabsTrigger };
