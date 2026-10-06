import { useCallback, useMemo, type ChangeEvent } from 'react';
import type { RecorderSceneSource } from '@daydream/shared';
import { FullHeightLayout } from '@/app/PageLayout';
import { Button } from '@/components/ui/button';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { recorderConfigurationLocked } from '../recorder-lifecycle-state';
import {
  croppedDimensions,
  reviewSourceSetup,
} from '../source-capture-geometry';
import { useSourceCaptureSetup } from '../use-source-capture-setup';
import { CaptureSourcesPanel } from './capture-sources-panel';
import { CaptureToolbar } from './capture-toolbar';
import type { CaptureWorkspaceProps } from './capture-workspace';
import { RecorderErrorAlert } from './recorder-error-alert';
import { RecordingInProgressPanel } from './recording-in-progress-panel';
import { SourceCaptureProperties } from './source-capture-properties';

export function SourceCaptureWorkspace(props: CaptureWorkspaceProps) {
  const setup = useSourceCaptureSetup();
  const locked =
    recorderConfigurationLocked(props.pendingLifecycleAction, props.status) ||
    setup.busy;
  const review = useMemo(
    () =>
      props.scene
        ? reviewSourceSetup(
            props.scene,
            props.captureSources,
            props.previewFramesBySceneSourceId,
          )
        : { scene: null, error: null },
    [props.scene, props.captureSources, props.previewFramesBySceneSourceId],
  );
  const editable = review.scene ?? props.scene;
  const source =
    editable?.sources.find(
      (source) => source.id === props.selectedSceneSourceId,
    ) ??
    editable?.sources[0] ??
    null;
  const frame = source
    ? props.previewFramesBySceneSourceId[source.id]
    : undefined;
  const defaults = setup.defaults;
  const needsReview = props.scene !== null && !props.scene.independentVideo;
  const dimensionsChanged =
    review.scene?.sources.some((source) => {
      const saved = props.scene?.sources.find(
        (candidate) => candidate.id === source.id,
      );
      return (
        saved?.sourceWidth !== source.sourceWidth ||
        saved?.sourceHeight !== source.sourceHeight
      );
    }) ?? false;
  const dirty =
    props.isDirty || setup.dirty || needsReview || dimensionsChanged;
  function updateSource(updated: RecorderSceneSource) {
    if (!editable || locked) return;
    props.onChangeScene({
      ...editable,
      sources: editable.sources.map((source) =>
        source.id === updated.id ? updated : source,
      ),
    });
  }
  function save() {
    if (!review.scene || review.error || !defaults) return;
    void setup.save({
      ...review.scene,
      desktopAudioSourceId: props.desktopAudioSourceId,
      microphoneAudioSourceId: props.microphoneAudioSourceId,
    });
  }
  const crop = source ? croppedDimensions(source) : null;
  const { setDefaults } = setup;
  const changeInterval = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      const value = Number(event.target.value);
      if (defaults && value > 0 && Number.isFinite(value))
        setDefaults({ ...defaults, fps: 1 / value });
    },
    [defaults, setDefaults],
  );
  const changeBitrate = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      const value = Number(event.target.value);
      if (defaults && Number.isInteger(value))
        setDefaults({ ...defaults, videoBitrateKbps: value });
    },
    [defaults, setDefaults],
  );
  return (
    <FullHeightLayout>
      <CaptureToolbar
        configurationLocked={
          locked || review.error !== null || defaults === null
        }
        isDirty={dirty}
        lifecycleLocked={props.pendingLifecycleAction !== null || setup.busy}
        loadStatus={props.loadStatus}
        pendingLifecycleAction={props.pendingLifecycleAction}
        previewStatus={props.previewStatus}
        saveStatus={setup.busy ? 'loading' : props.saveStatus}
        scene={props.scene}
        status={props.status}
        onPause={props.onPause}
        onResume={props.onResume}
        onSave={save}
        onStart={props.onStart}
        onStop={props.onStop}
      />
      {props.error || setup.error ? (
        <RecorderErrorAlert message={props.error ?? setup.error ?? ''} />
      ) : null}
      {setup.error && !setup.defaults ? (
        <Button onClick={setup.reload} size="sm" variant="outline">
          Retry recording settings
        </Button>
      ) : null}
      {props.status?.finalization ? (
        <p role="status" className="border-b px-4 py-2 text-xs">
          {props.status.finalization.action === 'stop'
            ? 'Saving recording'
            : 'Saving segment'}
        </p>
      ) : null}
      {props.status?.state === 'recording' ||
      props.status?.state === 'paused' ? (
        <p role="status" className="border-b px-4 py-2 text-xs">
          {props.status.state === 'recording'
            ? 'Recording in progress'
            : 'Recording paused'}
          . Source settings are locked until you stop.
        </p>
      ) : null}
      {needsReview ? (
        <p role="status" className="border-b px-4 py-2 text-xs">
          Review the source crops and output sizes, then save. Existing crops
          are retained; scale starts at 100%.
        </p>
      ) : null}
      {review.error ? (
        <p role="alert" className="text-destructive border-b px-4 py-2 text-xs">
          {review.error}
        </p>
      ) : null}
      <div className="grid min-h-0 flex-1 grid-cols-1 xl:grid-cols-[14rem_minmax(0,1fr)_20rem]">
        <div className="border-b xl:border-r xl:border-b-0">
          <CaptureSourcesPanel
            audioSources={props.audioSources}
            configurationLocked={locked}
            desktopAudioSourceId={props.desktopAudioSourceId}
            lifecycleLocked={props.pendingLifecycleAction !== null}
            loadStatus={props.loadStatus}
            microphoneAudioSourceId={props.microphoneAudioSourceId}
            pendingDuplicateCaptureSources={
              props.pendingDuplicateCaptureSources
            }
            onSelectAudioSource={props.onSelectAudioSource}
            onDeleteSceneSource={props.onDeleteSceneSource}
            onResolveDuplicateCaptureSource={
              props.onResolveDuplicateCaptureSource
            }
            onSelectCaptureSources={props.onSelectCaptureSources}
            onSelectSceneSource={props.onSelectSceneSource}
            scene={props.scene}
            selectedSceneSourceId={source?.id ?? null}
          />
        </div>
        <section
          aria-label="Selected source preview"
          className="flex min-w-0 flex-col gap-3 p-4"
        >
          <div className="flex justify-between gap-2 text-xs">
            <h2 className="font-semibold">
              {source?.displayName ?? 'Select a source'}
            </h2>
            <span className="text-muted-foreground">Separate video track</span>
          </div>
          {props.status?.state === 'recording' ? (
            <RecordingInProgressPanel
              recordingStats={props.recordingStats}
              recordingStatsError={props.recordingStatsError}
            />
          ) : frame && source && crop ? (
            <div
              className="relative w-full overflow-hidden border bg-black"
              style={{
                aspectRatio: `${source.sourceWidth}/${source.sourceHeight}`,
              }}
            >
              <img
                alt={`${source.displayName} preview`}
                src={frame.dataUrl}
                className="size-full object-fill"
              />
              <div
                aria-label="Capture region"
                className="pointer-events-none absolute border-2 border-sky-400"
                style={{
                  left: `${(source.crop.left / (source.sourceWidth ?? 1)) * 100}%`,
                  top: `${(source.crop.top / (source.sourceHeight ?? 1)) * 100}%`,
                  width: `${(crop.width / (source.sourceWidth ?? 1)) * 100}%`,
                  height: `${(crop.height / (source.sourceHeight ?? 1)) * 100}%`,
                }}
              />
            </div>
          ) : (
            <p className="text-muted-foreground flex min-h-48 items-center justify-center border text-xs">
              {source
                ? 'Waiting for source preview…'
                : 'Choose monitors to record.'}
            </p>
          )}
          {source ? (
            <p className="text-muted-foreground text-xs">
              Saved output: {source.rect.width} × {source.rect.height}. Each
              source is recorded and analyzed independently.
            </p>
          ) : null}
        </section>
        <aside className="border-t xl:border-t-0 xl:border-l">
          {source ? (
            <SourceCaptureProperties
              source={source}
              disabled={locked}
              onChange={updateSource}
            />
          ) : (
            <p className="text-muted-foreground p-3 text-xs">
              Select a source to set its crop and output size.
            </p>
          )}
        </aside>
      </div>
      <section
        aria-label="Shared recording settings"
        className="grid gap-3 border-t p-3 sm:grid-cols-2"
      >
        <Field>
          <FieldLabel htmlFor="capture-interval">
            Frame interval (seconds)
          </FieldLabel>
          <Input
            id="capture-interval"
            type="number"
            min={0.2}
            max={60}
            step={0.2}
            disabled={locked || !defaults}
            value={defaults ? Number((1 / defaults.fps).toFixed(3)) : ''}
            onChange={changeInterval}
          />
        </Field>
        <Field>
          <FieldLabel htmlFor="capture-quality">
            Video bitrate per source (kbps)
          </FieldLabel>
          <Input
            id="capture-quality"
            type="number"
            min={250}
            max={10000}
            step={250}
            disabled={locked || !defaults}
            value={defaults?.videoBitrateKbps ?? ''}
            onChange={changeBitrate}
          />
        </Field>
      </section>
    </FullHeightLayout>
  );
}
