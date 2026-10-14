import { useCallback, useEffect, useRef, useState } from 'react';
import type { RecallEvidence, RecallSourceKind } from '@daydream/shared';
import {
  listenToRecallStateChanged,
  resolveRecallEvidence,
} from '@/features/recall/recall-client';

export interface StudioEvidenceLocation {
  evidenceId: string | null;
  source: RecallSourceKind | null;
  seek: number | null;
}
type EvidenceState =
  | { status: 'loading' | 'unavailable'; evidence: null }
  | { status: 'ready'; evidence: RecallEvidence };

/** Publish private evidence only while a validated invalidation subscription owns the read. */
export function useStudioRecallEvidence(
  location: StudioEvidenceLocation,
  recordingId: string,
  privacyRevision: string,
) {
  const [revision, setRevision] = useState(0);
  const ownership = useRef(0);
  const [result, setResult] = useState<{
    key: string;
    state: EvidenceState;
  } | null>(null);
  const retry = useCallback(() => {
    ownership.current += 1;
    setRevision((current) => current + 1);
  }, []);
  const key = JSON.stringify([
    location,
    recordingId,
    privacyRevision,
    revision,
  ]);
  const { evidenceId, source, seek } = location;
  useEffect(() => {
    if (!evidenceId) return;
    const selectedEvidenceId = evidenceId;
    let active = true;
    let unsubscribe: (() => void) | undefined;
    const request = ++ownership.current;
    function invalidate() {
      if (!active) return;
      ownership.current += 1;
      setResult({ key, state: { status: 'loading', evidence: null } });
      setRevision((current) => current + 1);
    }
    function disconnect() {
      if (!active) return;
      active = false;
      ownership.current += 1;
      const cleanup = unsubscribe;
      unsubscribe = undefined;
      cleanup?.();
      setResult({ key, state: { status: 'unavailable', evidence: null } });
    }
    async function load() {
      try {
        // Events received while listen() is pending also invalidate this request.
        const cleanup = await listenToRecallStateChanged(
          invalidate,
          disconnect,
        );
        if (!active || request !== ownership.current) {
          cleanup();
          return;
        }
        unsubscribe = cleanup;
        const evidence = await resolveRecallEvidence({
          evidenceId: selectedEvidenceId,
        });
        if (!active || request !== ownership.current) return;
        const eligible =
          evidence !== null &&
          evidence.recordingId === recordingId &&
          (source === null || source === evidence.sourceKind) &&
          (seek === null || seek === evidence.startOffsetMs);
        setResult({
          key,
          state: eligible
            ? { status: 'ready', evidence }
            : { status: 'unavailable', evidence: null },
        });
      } catch {
        if (active && request === ownership.current) disconnect();
      }
    }
    void load();
    return () => {
      active = false;
      ownership.current += 1;
      unsubscribe?.();
    };
  }, [evidenceId, source, seek, recordingId, key]);
  return {
    state:
      evidenceId === null
        ? null
        : result?.key === key
          ? result.state
          : ({ status: 'loading', evidence: null } satisfies EvidenceState),
    retry,
  };
}
