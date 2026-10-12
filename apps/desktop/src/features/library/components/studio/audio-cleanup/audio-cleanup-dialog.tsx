import { SharedDisclosure } from '@/components/shared-disclosure';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Label } from '@/components/ui/label';
import { Progress } from '@/components/ui/progress';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Slider } from '@/components/ui/slider';
import { AudioCleanupComparison } from './audio-cleanup-preview';
import type { AudioCleanupController } from './use-audio-cleanup';

export interface AudioCleanupDialogProps {
  controller: AudioCleanupController;
  onRegenerate: () => void;
  unavailableReason: string | null;
  scopeLabel?: string;
}
export function AudioCleanupDialog({
  controller: c,
  onRegenerate,
  unavailableReason,
  scopeLabel,
}: AudioCleanupDialogProps) {
  const running = c.state?.phase === 'running';
  const disabled = c.busy !== null || running;
  function opened(value: boolean) {
    if (!value) c.close();
  }
  function mode(value: string) {
    if (value === 'off' || value === 'auto')
      c.change({ ...c.draft, mode: value });
  }
  function sensitivity(values: number[]) {
    const value = values[0];
    if (value !== undefined)
      c.change({
        ...c.draft,
        matchThreshold: Math.round((0.99 - value * 0.0089) * 100) / 100,
      });
  }
  function threshold(values: number[]) {
    const value = values[0];
    if (value !== undefined)
      c.change({ ...c.draft, matchThreshold: value / 100 });
  }
  function amount(values: number[]) {
    const value = values[0];
    if (value !== undefined) c.change({ ...c.draft, amount: value / 100 });
  }
  function regenerate() {
    c.close();
    onRegenerate();
  }
  const status =
    c.state?.phase === 'ready'
      ? c.state.matchedMs === 0
        ? 'No speaker bleed detected. Microphone audio is unchanged.'
        : 'Cleaned microphone audio is ready for playback.'
      : c.state?.phase === 'invalidated'
        ? 'Sources or privacy settings changed. Apply cleanup again to rebuild the audio.'
        : c.state?.phase === 'cancelled'
          ? 'Cleanup stopped. Retry resumes completed blocks.'
          : c.state?.phase === 'failed'
            ? 'Cleanup failed. Your previously applied audio is still selected.'
            : null;
  return (
    <Dialog open={c.opened} onOpenChange={opened}>
      <DialogContent
        className="flex max-h-[90dvh] flex-col overflow-hidden sm:max-w-lg"
        data-studio-preserve-selection
      >
        <DialogHeader className="shrink-0">
          <DialogTitle>Remove speaker bleed</DialogTitle>
          <DialogDescription>
            Use the desktop recording to remove speaker audio from the
            microphone. Processing stays on this machine; the original
            recordings are preserved.
          </DialogDescription>
          {scopeLabel ? (
            <p className="text-muted-foreground text-xs">
              Applies to the whole recording: {scopeLabel}.
            </p>
          ) : null}
        </DialogHeader>
        <div className="min-h-0 space-y-5 overflow-y-auto">
          <div className="space-y-2">
            <Label htmlFor="cleanup-mode">Cancellation</Label>
            <Select
              value={c.draft.mode}
              onValueChange={mode}
              disabled={disabled}
            >
              <SelectTrigger id="cleanup-mode">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="off">Off</SelectItem>
                <SelectItem value="auto">Auto</SelectItem>
              </SelectContent>
            </Select>
          </div>
          {c.draft.mode === 'auto' ? (
            <>
              <div className="space-y-3">
                <Label>Sensitivity</Label>
                <Slider
                  aria-label="Speaker bleed sensitivity"
                  min={0}
                  max={100}
                  step={1}
                  value={[Math.round((0.99 - c.draft.matchThreshold) / 0.0089)]}
                  onValueChange={sensitivity}
                  disabled={disabled}
                />
                <p className="text-muted-foreground text-xs">
                  Higher sensitivity cancels weaker matches. Lower it if your
                  voice sounds affected.
                </p>
              </div>
              <SharedDisclosure label="Advanced tuning">
                <div className="space-y-5">
                  <div className="space-y-3">
                    <Label>
                      Match threshold · {c.draft.matchThreshold.toFixed(2)}
                    </Label>
                    <Slider
                      aria-label="Match threshold"
                      min={10}
                      max={99}
                      step={1}
                      value={[Math.round(c.draft.matchThreshold * 100)]}
                      onValueChange={threshold}
                      disabled={disabled}
                    />
                    <p className="text-muted-foreground text-xs">
                      Similarity between the feeds after timing alignment; this
                      is not a probability.
                    </p>
                  </div>
                  <div className="space-y-3">
                    <Label>
                      Cancellation amount · {Math.round(c.draft.amount * 100)}%
                    </Label>
                    <Slider
                      aria-label="Cancellation amount"
                      min={0}
                      max={100}
                      step={5}
                      value={[Math.round(c.draft.amount * 100)]}
                      onValueChange={amount}
                      disabled={disabled}
                    />
                  </div>
                </div>
              </SharedDisclosure>
            </>
          ) : null}
          {unavailableReason ? (
            <p role="status" className="text-muted-foreground text-sm">
              {unavailableReason}
            </p>
          ) : null}
          {!c.state?.referenceAvailable && c.state ? (
            <p role="status" className="text-muted-foreground text-sm">
              Both desktop and microphone audio are needed for cancellation.
            </p>
          ) : null}
          <Button
            type="button"
            variant="outline"
            onClick={c.compare}
            disabled={
              disabled ||
              unavailableReason !== null ||
              !c.state?.referenceAvailable
            }
          >
            {' '}
            {c.busy === 'preview'
              ? 'Preparing comparison…'
              : 'Preview selected range'}
          </Button>
          {c.preview ? (
            <AudioCleanupComparison
              key={c.preview.sessionId}
              preview={c.preview}
            />
          ) : null}
          {running ? (
            <div className="space-y-2" role="status">
              <p className="text-sm">
                Removing speaker bleed ·{' '}
                {Math.round((c.state?.progress ?? 0) * 100)}%
              </p>
              <Progress
                value={(c.state?.progress ?? 0) * 100}
                aria-label="Audio cleanup progress"
              />
              <p className="text-muted-foreground text-xs">
                Current audio remains available while cleanup runs.
              </p>
            </div>
          ) : null}
          {status ? (
            <p role="status" className="text-muted-foreground text-sm">
              {status}
            </p>
          ) : null}
          {c.state?.transcriptsOutdated ? (
            <div className="space-y-2">
              <p className="text-muted-foreground text-sm">
                Microphone transcripts use previous audio settings.
              </p>
              <Button
                variant="outline"
                onClick={regenerate}
                disabled={disabled || c.state.phase === 'invalidated'}
              >
                Regenerate microphone transcripts
              </Button>
            </div>
          ) : null}
          {c.error ? (
            <p role="alert" className="text-destructive text-sm">
              {c.error}
            </p>
          ) : null}
        </div>
        <DialogFooter className="shrink-0">
          <Button
            variant="ghost"
            onClick={c.close}
            disabled={c.busy === 'apply' || c.busy === 'cancel'}
          >
            Close
          </Button>
          {running ? (
            <Button
              variant="outline"
              onClick={c.cancel}
              disabled={c.busy !== null}
            >
              Cancel cleanup
            </Button>
          ) : (
            <Button
              onClick={c.apply}
              disabled={
                c.busy !== null ||
                !c.state ||
                (c.draft.mode === 'auto' &&
                  (unavailableReason !== null || !c.state.referenceAvailable))
              }
            >
              {c.state?.phase === 'failed' || c.state?.phase === 'cancelled'
                ? 'Retry cleanup'
                : 'Apply'}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
