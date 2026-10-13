import { useCallback, useEffect, useRef, useState } from 'react';
import type { RecallJob } from '@daydream/shared';
import { controlRecallJob, regenerateRecallDay } from '../recall-client';
import { recallTimezone } from '../recall-format';

export function summaryFailureMessage(error: unknown): string {
  const code =
    typeof error === 'string'
      ? error
      : typeof error === 'object' && error !== null && 'code' in error
        ? error.code
        : null;
  switch (code) {
    case 'recall_provider_auth_failed':
    case 'recall_credential_missing':
    case 'recall_credential_unavailable':
    case 'recall_provider_subscription_reconnect':
      return 'The summary provider could not authenticate. Check summary settings and try again.';
    case 'recall_provider_network':
    case 'recall_provider_timeout':
    case 'recall_provider_server_failed':
    case 'recall_provider_rate_limited':
      return 'The summary provider is unavailable. Try again when the connection or provider recovers.';
    case 'recall_input_changed':
    case 'recall_profile_changed':
      return 'Recorded content or summary settings changed. Generate again to use the current sources.';
    case 'recall_generation_disabled':
    case 'recall_profile_invalid':
      return 'Summary setup is incomplete. Review summary settings and try again.';
    default:
      return 'Summary generation could not finish. Try again or review summary settings.';
  }
}

type RequestState =
  | { status: 'idle'; job: RecallJob | null }
  | { status: 'starting'; job: RecallJob | null }
  | { status: 'error'; job: RecallJob | null; message: string };

export function useDaySummaryGeneration(
  date: string,
  jobs: RecallJob[],
  refresh: () => void,
) {
  const [request, setRequest] = useState<RequestState>({
    status: 'idle',
    job: null,
  });
  const active = useRef(false);
  const inFlight = useRef(false);
  useEffect(() => {
    active.current = true;
    return () => {
      active.current = false;
    };
  }, []);
  const currentJobs = jobs.filter(
    (job) => job.kind === 'generation' && job.dayId === date,
  );
  const latest = currentJobs.reduce<RecallJob | null>(
    (newest, job) =>
      newest === null || job.updatedAtMs > newest.updatedAtMs ? job : newest,
    null,
  );
  const returned = request.job;
  const persisted = currentJobs.find(
    (candidate) => candidate.id === returned?.id,
  );
  const tracked = returned
    ? persisted && persisted.updatedAtMs > returned.updatedAtMs
      ? persisted
      : returned
    : latest;
  const job =
    tracked &&
    latest &&
    !['pending', 'running', 'paused'].includes(tracked.status) &&
    latest.updatedAtMs > tracked.updatedAtMs
      ? latest
      : tracked;
  const working =
    request.status === 'starting' ||
    job?.status === 'pending' ||
    job?.status === 'running';
  const run = useCallback(
    async (action: () => Promise<RecallJob>) => {
      if (inFlight.current) return;
      inFlight.current = true;
      setRequest((state) => ({ status: 'starting', job: state.job }));
      try {
        const result = await action();
        if (!active.current) return;
        setRequest({ status: 'idle', job: result });
        refresh();
      } catch (error) {
        if (active.current)
          setRequest((state) => ({
            status: 'error',
            job: state.job,
            message: summaryFailureMessage(error),
          }));
      } finally {
        inFlight.current = false;
      }
    },
    [refresh],
  );
  const generate = useCallback(() => {
    if (working) return;
    void run(() =>
      regenerateRecallDay({ dayId: date, timezone: recallTimezone }),
    );
  }, [date, run, working]);
  const resume = useCallback(() => {
    if (job?.status !== 'paused') return;
    void run(() => controlRecallJob({ jobId: job.id, action: 'resume' }));
  }, [job, run]);
  const error =
    request.status === 'error'
      ? request.message
      : job?.status === 'failed'
        ? summaryFailureMessage(job.errorCode)
        : null;
  const message =
    request.status === 'starting'
      ? 'Starting summary generation…'
      : job?.status === 'pending'
        ? job.errorCode
          ? 'Summary queued to retry after a provider interruption.'
          : 'Summary queued.'
        : job?.status === 'running'
          ? 'Generating summary…'
          : job?.status === 'paused'
            ? 'Summary generation paused.'
            : job?.status === 'succeeded'
              ? 'Summary generation completed.'
              : job?.status === 'cancelled'
                ? 'Summary generation cancelled. You can generate again.'
                : null;
  return {
    generate,
    resume,
    working,
    paused: job?.status === 'paused',
    error,
    message,
  };
}
