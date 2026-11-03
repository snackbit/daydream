import { invoke, decodeResponse } from '@/lib/diagnostics/invoke';

export async function getPlaybackDecoderWarmupUrl(): Promise<string> {
  const command = 'getPlaybackDecoderWarmupUrl';
  return decodeResponse(command, await invoke<unknown>(command), (value) => {
    if (typeof value !== 'string')
      throw new Error('Invalid playback warmup URL');
    const url = new URL(value);
    if (
      url.protocol !== 'http:' ||
      !['127.0.0.1', 'localhost', '[::1]'].includes(url.hostname)
    )
      throw new Error('Playback warmup must remain local');
    return url.href;
  });
}
