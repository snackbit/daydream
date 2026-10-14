import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
} from 'react';
import type { RecorderSceneSource } from '@daydream/shared';
import { FullHeightLayout } from '@/app/PageLayout';
import { Button } from '@/components/ui/button';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from '@/components/ui/sheet';
import {
  recorderConfigurationLocked,
  recorderLifecycleLocked,
} from '../recorder-lifecycle-state';
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
import { SourceCropEditor } from './source-crop-editor';

export function SourceCaptureWorkspace(props: CaptureWorkspaceProps) {
  const [compact, setCompact] = useState(() => window.innerWidth < 1200);
  const [propertiesOpen, setPropertiesOpen] = useState(false);
  const [regionId, setRegionId] = useState<string | null>(null);
  const [customId, setCustomId] = useState<string | null>(null);
  const propertiesRef = useRef<HTMLDivElement>(null);
  const propertiesTriggerRef = useRef<HTMLButtonElement>(null);
  const interactedOutsideProperties = useRef(false);
  const changePropertiesOpen = useCallback(
    (open: boolean) => {
      if (open === propertiesOpen) return;
      setPropertiesOpen(open);
      const restoreTrigger = !interactedOutsideProperties.current;
      interactedOutsideProperties.current = false;
      requestAnimationFrame(() => {
        if (open) {
          propertiesRef.current?.focus();
          interactedOutsideProperties.current = false;
        } else if (restoreTrigger) propertiesTriggerRef.current?.focus();
      });
    },
    [propertiesOpen],
  );
  const handlePropertiesInteractOutside = useCallback(
    (event: Event) => {
      if (!propertiesOpen) event.preventDefault();
      else interactedOutsideProperties.current = true;
    },
    [propertiesOpen],
  );
  const preventClosedAutoFocus = useCallback(
    (event: Event) => {
      if (!propertiesOpen) event.preventDefault();
    },
    [propertiesOpen],
  );
  useEffect(() => {
    const query = window.matchMedia('(max-width: 1199px)');
    function update() {
      setCompact(query.matches);
    }
    update();
    query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  }, []);
  const setup = useSourceCaptureSetup();
  const locked =
    recorderConfigurationLocked(props.pendingLifecycleAction, props.status) ||
    props.captureSelectionPending === true ||
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
  function start() {
    if (!review.scene || review.error || !defaults || locked) return;
    props.onStart({
      ...defaults,
      scene: {
        ...review.scene,
        desktopAudioSourceId: props.desktopAudioSourceId,
        microphoneAudioSourceId: props.microphoneAudioSourceId,
      },
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
  const properties = source ? (
    <SourceCaptureProperties
      source={source}
      disabled={locked}
      onChange={updateSource}
      regionId={regionId}
      customId={customId}
      onRegionChange={setRegionId}
      onCustomChange={setCustomId}
    />
  ) : (
    <p className="text-muted-foreground p-6 text-sm">
      Select a source to set its crop and output size.
    </p>
  );
  return (
    <FullHeightLayout className="bg-background text-foreground">
      <CaptureToolbar
        captureSelectionPending={props.captureSelectionPending === true}
        setupBlocker={
          review.error ??
          (!defaults
            ? 'Load recording settings before starting.'
            : setup.busy
              ? 'Saving source setup…'
              : null)
        }
        configurationLocked={
          locked || review.error !== null || defaults === null
        }
        isDirty={dirty}
        lifecycleLocked={
          recorderLifecycleLocked(props.pendingLifecycleAction, props.status) ||
          setup.busy
        }
        loadStatus={props.loadStatus}
        pendingLifecycleAction={props.pendingLifecycleAction}
        previewStatus={props.previewStatus}
        saveStatus={setup.busy ? 'loading' : props.saveStatus}
        scene={props.scene}
        status={props.status}
        onPause={props.onPause}
        onResume={props.onResume}
        onSave={save}
        onStart={start}
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
      <div className="grid min-h-0 flex-1 grid-cols-1 gap-6 px-5 pb-6 min-[1200px]:grid-cols-[18rem_minmax(0,1fr)_20rem] lg:px-8">
        <div className="bg-muted rounded-panel min-w-0">
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
          className="bg-muted rounded-workspace flex min-w-0 flex-col gap-6 p-6 lg:p-8"
        >
          <div className="flex flex-wrap justify-between gap-2 text-sm">
            <h2 className="font-medium">
              {source?.displayName ?? 'Select a source'}
            </h2>
            <span className="text-muted-foreground text-xs">
              Separate video track
            </span>
            {compact ? (
              <Sheet
                modal={false}
                open={propertiesOpen}
                onOpenChange={changePropertiesOpen}
              >
                <SheetTrigger asChild>
                  <Button ref={propertiesTriggerRef} variant="outline">
                    Source output
                  </Button>
                </SheetTrigger>
                <SheetContent
                  hidden={!propertiesOpen}
                  ref={propertiesRef}
                  onOpenAutoFocus={preventClosedAutoFocus}
                  onInteractOutside={handlePropertiesInteractOutside}
                  forceMount
                  className="w-full max-w-[28rem] gap-0 overflow-y-auto p-0 data-[state=closed]:hidden sm:max-w-[28rem]"
                >
                  <SheetHeader className="p-6">
                    <SheetTitle>Source output settings</SheetTitle>
                    <SheetDescription>
                      Set the capture area and output size for the selected
                      source.
                    </SheetDescription>
                  </SheetHeader>
                  {properties}
                </SheetContent>
              </Sheet>
            ) : null}
          </div>
          {props.status?.state === 'recording' ? (
            <RecordingInProgressPanel
              recordingStats={props.recordingStats}
              recordingStatsError={props.recordingStatsError}
            />
          ) : source && source.sourceWidth && source.sourceHeight ? (
            <SourceCropEditor
              key={source.id}
              source={source}
              frame={frame}
              unavailable={props.previewStatus?.state === 'error'}
              disabled={locked}
              onChange={updateSource}
            />
          ) : (
            <p className="text-muted-foreground flex min-h-64 items-center justify-center rounded-lg border p-6 text-sm">
              {source
                ? 'Waiting for source preview…'
                : 'Choose monitors to record.'}
            </p>
          )}
          {source ? (
            <p className="text-muted-foreground text-xs">
              Crop: {crop?.width} × {crop?.height} · Saved output:{' '}
              {source.rect.width} × {source.rect.height}
            </p>
          ) : null}
        </section>
        {!compact ? (
          <aside className="bg-muted rounded-panel min-w-0">{properties}</aside>
        ) : null}
      </div>
      <section
        aria-label="Shared recording settings"
        className="bg-muted rounded-panel mx-5 mb-6 grid gap-6 p-6 sm:grid-cols-2 lg:mx-8 lg:p-8"
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
