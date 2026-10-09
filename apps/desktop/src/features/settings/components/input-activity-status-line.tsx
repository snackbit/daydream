import type { InputActivityStatus } from '@daydream/shared';
import { CheckCircle2, CircleHelp, Loader2, ShieldAlert } from 'lucide-react';
import { Button } from '@/components/ui/button';

export interface InputActivityStatusLineProps {
  onRefresh: () => void;
  onRepair: () => void;
  repairMessage: string | null;
  repairing: boolean;
  status: InputActivityStatus | null;
}

export function InputActivityStatusLine({
  onRefresh,
  onRepair,
  repairMessage,
  repairing,
  status,
}: InputActivityStatusLineProps) {
  const Icon = status?.available
    ? CheckCircle2
    : status
      ? ShieldAlert
      : CircleHelp;
  const canRepair = status && !status.available && status.repairAvailable;
  const needsLogout =
    status &&
    !status.available &&
    status.reason === 'permission-denied' &&
    status.userInInputGroup;
  return (
    <section
      aria-label="Input activity access"
      className="text-control grid gap-2"
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div
          role="status"
          className="flex min-w-0 flex-wrap items-center gap-2"
        >
          <Icon aria-hidden="true" className="text-muted-foreground size-4" />
          <span className="font-medium">
            {status?.available
              ? 'Input activity available'
              : status
                ? 'Input activity unavailable'
                : 'Input activity status unknown'}
          </span>
          {status?.platform === 'linux' ? (
            <span className="text-muted-foreground text-metadata">
              {status.readableDeviceCount}/{status.candidateDeviceCount} input
              devices readable.
            </span>
          ) : null}
        </div>
        <div className="flex flex-wrap gap-2">
          <Button onClick={onRefresh} size="sm" type="button" variant="outline">
            Recheck
          </Button>
          {canRepair ? (
            <Button
              disabled={repairing}
              onClick={onRepair}
              size="sm"
              type="button"
              variant="secondary"
            >
              {repairing ? (
                <Loader2 aria-hidden="true" className="size-4 animate-spin" />
              ) : null}
              {repairing ? 'Repairing...' : 'Repair permissions'}
            </Button>
          ) : null}
        </div>
      </div>
      {!status ? (
        <p className="text-muted-foreground text-metadata">
          Daydream has not checked keyboard and mouse access yet.
        </p>
      ) : !status.available ? (
        <p className="text-destructive text-control" role="status">
          {status.message}
        </p>
      ) : null}
      {needsLogout ? (
        <p className="text-control">
          Log out and back in, then reopen Daydream.
        </p>
      ) : null}
      {repairMessage ? (
        <p className="text-control" role="status">
          {repairMessage}
        </p>
      ) : null}
    </section>
  );
}
