import { useRef, useState, type SyntheticEvent } from 'react';
import type { AudioCleanupPreview } from '@daydream/shared';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';

export interface AudioCleanupPreviewProps {
  preview: AudioCleanupPreview;
}
export function AudioCleanupComparison({ preview }: AudioCleanupPreviewProps) {
  const [variant, setVariant] = useState<'original' | 'cleaned'>('cleaned');
  const audio = useRef<HTMLAudioElement>(null);
  const [mediaError, setMediaError] = useState(false);
  const position = useRef(0);
  function choose(value: string) {
    if (value !== 'original' && value !== 'cleaned') return;
    position.current = audio.current?.currentTime ?? 0;
    audio.current?.pause();
    setVariant(value);
    setMediaError(false);
  }
  function loaded(event: SyntheticEvent<HTMLAudioElement>) {
    event.currentTarget.currentTime = Math.min(
      position.current,
      Math.max(0, event.currentTarget.duration - 0.05),
    );
  }
  function failed() {
    setMediaError(true);
  }
  return (
    <div className="space-y-3">
      <Tabs
        value={variant}
        onValueChange={choose}
        aria-label="Microphone comparison"
      >
        <TabsList>
          <TabsTrigger value="original">Original</TabsTrigger>
          <TabsTrigger value="cleaned">Cleaned</TabsTrigger>
        </TabsList>
      </Tabs>
      <audio
        ref={audio}
        controls
        preload="metadata"
        aria-label={`${variant === 'original' ? 'Original' : 'Cleaned'} microphone preview`}
        className="h-10 w-full"
        src={variant === 'original' ? preview.originalUrl : preview.cleanedUrl}
        onLoadedMetadata={loaded}
        onError={failed}
      />
      <p className="text-muted-foreground text-xs">
        Microphone only · {Math.round(preview.durationMs / 1000)} seconds.
        Switching keeps the listening position.
      </p>
      {mediaError ? (
        <p role="alert" className="text-destructive text-sm">
          This comparison is no longer available. Preview again.
        </p>
      ) : null}
    </div>
  );
}
