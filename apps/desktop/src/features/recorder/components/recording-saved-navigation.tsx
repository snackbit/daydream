import { useEffect, useState } from 'react';
import { Link, useRouter } from '@tanstack/react-router';
import { Button } from '@/components/ui/button';
import { listenToRecordingSaved } from '../recorder-client';
import { useRecorderWorkspaceStore } from '../recorder-workspace-store';

export function RecordingSavedNavigation() {
  const router = useRouter();
  const [available, setAvailable] = useState<string | null>(null);
  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    let pending: {
      recordingId: string;
      location: string;
      navigated: boolean;
    } | null = null;
    const seen = new Set<string>();
    const unsubscribeStore = useRecorderWorkspaceStore.subscribe(
      (state, previous) => {
        if (
          state.pendingLifecycleAction === 'stop' &&
          previous.pendingLifecycleAction !== 'stop' &&
          state.status?.activeRecordingId
        )
          pending = {
            recordingId: state.status.activeRecordingId,
            location: router.state.location.href,
            navigated: false,
          };
      },
    );
    const unsubscribeRoute = router.subscribe('onBeforeNavigate', () => {
      if (pending) pending.navigated = true;
    });
    void listenToRecordingSaved((saved) => {
      if (!active || seen.has(saved.recordingId)) return;
      seen.add(saved.recordingId);
      const owner = pending;
      pending = null;
      if (
        owner?.recordingId === saved.recordingId &&
        !owner.navigated &&
        router.state.location.href === owner.location
      ) {
        void router.navigate({
          to: '/library/$recordingId',
          params: { recordingId: saved.recordingId },
          search: { evidenceId: null, source: null, seek: null },
        });
      } else setAvailable(saved.recordingId);
    })
      .then((cleanup) => {
        if (active) unlisten = cleanup;
        else cleanup();
      })
      .catch(() => undefined);
    return () => {
      active = false;
      unlisten?.();
      unsubscribeStore();
      unsubscribeRoute();
    };
  }, [router]);
  function dismiss() {
    setAvailable(null);
  }
  if (!available) return null;
  return (
    <div
      role="status"
      className="bg-muted flex items-center justify-between gap-3 border-b px-5 py-2 text-sm"
    >
      Recording saved.
      <Button asChild size="sm" variant="outline">
        <Link
          to="/library/$recordingId"
          params={{ recordingId: available }}
          search={{ evidenceId: null, source: null, seek: null }}
          onClick={dismiss}
        >
          Open recording
        </Link>
      </Button>
      <Button size="sm" variant="ghost" onClick={dismiss}>
        Dismiss
      </Button>
    </div>
  );
}
