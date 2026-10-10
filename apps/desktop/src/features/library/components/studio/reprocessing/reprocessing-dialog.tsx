import { useCallback, useMemo } from 'react';
import type {
  ReprocessingOperation,
  ReprocessingOperationInfo,
} from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { formatTimelineOffset } from '../inspector/inspector-format';
import type { StudioReprocessingController } from './use-studio-reprocessing';

export const operationLabels: Record<ReprocessingOperation, string> = {
  transcription: 'Transcription',
  ocr: 'Screen text (OCR)',
  'screen-observations': 'Screen observations',
};
export interface ReprocessingDialogProps {
  controller: StudioReprocessingController;
}
export function ReprocessingDialog({ controller }: ReprocessingDialogProps) {
  const changeOpen = useCallback(
    (open: boolean) => {
      if (!open) controller.close();
    },
    [controller],
  );
  const selected = useMemo(
    () =>
      controller.request?.operations ??
      controller.preview?.operations
        .filter((info) => info.enabled)
        .map((info) => info.operation) ??
      [],
    [controller.request?.operations, controller.preview?.operations],
  );
  const toggle = useCallback(
    (operation: ReprocessingOperation, checked: boolean) => {
      controller.operations(
        checked
          ? [...selected, operation]
          : selected.filter((value) => value !== operation),
      );
    },
    [controller, selected],
  );
  return (
    <Dialog open={controller.request !== null} onOpenChange={changeOpen}>
      <DialogContent
        data-studio-preserve-selection
        className="max-h-[85vh] overflow-auto"
      >
        <DialogHeader>
          <DialogTitle>Reprocess sources</DialogTitle>
          <DialogDescription>
            Use current settings to replace these results. Existing content
            stays available until each replacement succeeds.
          </DialogDescription>
        </DialogHeader>
        {!controller.preview && !controller.error ? (
          <p role="status">Preparing the processing range…</p>
        ) : null}
        {controller.preview ? (
          <>
            <div className="space-y-3">
              {controller.preview.operations.map((info) => (
                <OperationChoice
                  key={info.operation}
                  info={info}
                  checked={selected.includes(info.operation)}
                  disabled={controller.pending}
                  onToggle={toggle}
                />
              ))}
            </div>
            <div className="max-h-52 space-y-2 overflow-auto rounded-lg border p-3">
              <p className="text-muted-foreground text-xs">
                Effective ranges include complete overlapping utterances for
                audio.
              </p>
              {controller.preview.scopes.map((scope) => (
                <p key={scope.id} className="text-sm">
                  {scope.sourceLabel} · {operationLabels[scope.operation]} ·{' '}
                  {formatTimelineOffset(scope.startOffsetMs)}–
                  {formatTimelineOffset(scope.endOffsetMs)}
                </p>
              ))}
            </div>
            {controller.preview.blockedReason ? (
              <p role="status" className="text-muted-foreground text-sm">
                {controller.preview.blockedReason}
              </p>
            ) : null}
          </>
        ) : null}
        {controller.error ? (
          <p role="alert" className="text-destructive text-sm">
            {controller.error}
          </p>
        ) : null}
        <DialogFooter>
          <Button
            variant="ghost"
            onClick={controller.close}
            disabled={controller.pending}
          >
            Cancel
          </Button>
          <Button
            onClick={controller.start}
            disabled={
              controller.pending ||
              !controller.preview ||
              controller.preview.blockedReason !== null ||
              controller.preview.scopes.length === 0
            }
          >
            {controller.pending ? 'Starting…' : 'Reprocess'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
function OperationChoice({
  info,
  checked,
  disabled,
  onToggle,
}: {
  info: ReprocessingOperationInfo;
  checked: boolean;
  disabled: boolean;
  onToggle: (operation: ReprocessingOperation, checked: boolean) => void;
}) {
  const toggle = useCallback(
    (value: boolean | 'indeterminate') =>
      onToggle(info.operation, value === true),
    [info.operation, onToggle],
  );
  return (
    <label className="flex items-start gap-3">
      <Checkbox
        aria-label={operationLabels[info.operation]}
        checked={checked}
        onCheckedChange={toggle}
        disabled={disabled || !info.enabled}
      />
      <span className="space-y-1 text-sm">
        <span className="block font-medium">
          {operationLabels[info.operation]}
        </span>
        <span className="text-muted-foreground block text-xs">
          {info.blockedReason ?? info.providerLabel}
          {info.remote
            ? ' · Remote provider receives the selected content'
            : ''}
        </span>
      </span>
    </label>
  );
}
