import { useCallback, type ReactNode } from 'react';
import type { ReprocessingTarget } from '@daydream/shared';
import { MoreHorizontal, RefreshCw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from '@/components/ui/context-menu';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';

export interface ReprocessingMenuProps {
  targets: ReprocessingTarget[];
  onReprocess: (targets: ReprocessingTarget[]) => void;
  label: string;
  children?: ReactNode;
  onOpenChange?: ((open: boolean) => void) | undefined;
}
export function ReprocessingDropdown({
  targets,
  onReprocess,
  label,
  onOpenChange,
}: ReprocessingMenuProps) {
  const reprocess = useCallback(
    () => onReprocess(targets),
    [targets, onReprocess],
  );
  return (
    <DropdownMenu {...(onOpenChange ? { onOpenChange } : {})}>
      <DropdownMenuTrigger asChild>
        <Button
          aria-label={`${label} actions`}
          data-library-action
          size="icon-xs"
          variant="ghost"
        >
          <MoreHorizontal aria-hidden="true" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent data-studio-preserve-selection align="end">
        <DropdownMenuItem onSelect={reprocess} disabled={targets.length === 0}>
          <RefreshCw />
          Reprocess {label.toLowerCase()}…
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
export function ReprocessingContextMenu({
  targets,
  onReprocess,
  label,
  children,
  onOpenChange,
}: ReprocessingMenuProps) {
  const reprocess = useCallback(
    () => onReprocess(targets),
    [targets, onReprocess],
  );
  return (
    <ContextMenu {...(onOpenChange ? { onOpenChange } : {})}>
      <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
      <ContextMenuContent data-studio-preserve-selection className="z-70">
        <ContextMenuItem onSelect={reprocess} disabled={targets.length === 0}>
          <RefreshCw />
          Reprocess {label.toLowerCase()}…
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}
