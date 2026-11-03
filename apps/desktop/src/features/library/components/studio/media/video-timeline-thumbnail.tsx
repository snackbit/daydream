import { useCallback, useState } from 'react';
import { ImageOff } from 'lucide-react';
import { Skeleton } from '@/components/ui/skeleton';

export interface VideoTimelineThumbnailProps {
  url: string;
}

export function VideoTimelineThumbnail({ url }: VideoTimelineThumbnailProps) {
  const [failed, setFailed] = useState(false);
  const [loaded, setLoaded] = useState(false);
  const handleError = useCallback(() => setFailed(true), []);
  const handleLoad = useCallback(() => setLoaded(true), []);
  return (
    <span className="bg-muted relative h-full min-w-0 flex-1 overflow-hidden border-r last:border-r-0">
      {failed ? (
        <span
          role="img"
          aria-label="Thumbnail unavailable"
          className="text-muted-foreground flex h-full items-center justify-center gap-1.5 px-2 text-xs"
          title="This preview could not be loaded."
        >
          <ImageOff aria-hidden="true" className="size-4 shrink-0" />
          <span className="truncate">Preview unavailable</span>
        </span>
      ) : (
        <>
          {!loaded ? (
            <Skeleton aria-hidden className="absolute inset-0 rounded-none" />
          ) : null}
          <img
            alt=""
            aria-label="Video timeline thumbnail"
            className="h-full w-full object-cover"
            draggable={false}
            onError={handleError}
            onLoad={handleLoad}
            src={url}
          />
        </>
      )}
    </span>
  );
}
