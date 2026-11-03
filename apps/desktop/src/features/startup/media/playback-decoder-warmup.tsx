import { useEffect } from 'react';
import { reportFrontendFailure } from '@/lib/diagnostics/failure-logging';
import { getPlaybackDecoderWarmupUrl } from './playback-warmup-client';

/** Initialize WebKit's local H.264 pipeline before the first Studio opening. */
export function PlaybackDecoderWarmup() {
  useEffect(() => {
    let active = true;
    const video = document.createElement('video');
    function failed(error: unknown) {
      if (active) reportFrontendFailure('frontend_action', 'execute', error);
      release();
    }
    function mediaFailed() {
      failed(new Error('Synthetic playback warmup could not decode'));
    }
    function release() {
      active = false;
      video.removeEventListener('loadeddata', warm);
      video.removeEventListener('error', mediaFailed);
      video.pause();
      video.removeAttribute('src');
      video.load();
    }
    function warm() {
      const canvas = document.createElement('canvas');
      canvas.width = video.videoWidth;
      canvas.height = video.videoHeight;
      try {
        canvas.getContext('2d')?.drawImage(video, 0, 0);
        performance.mark('studio.decoder.warm');
      } finally {
        release();
      }
    }
    video.muted = true;
    video.preload = 'auto';
    video.addEventListener('loadeddata', warm, { once: true });
    video.addEventListener('error', mediaFailed, { once: true });
    void getPlaybackDecoderWarmupUrl()
      .then((source) => {
        if (!active) return;
        video.src = source;
        video.load();
      })
      .catch(failed);
    return release;
  }, []);
  return null;
}
