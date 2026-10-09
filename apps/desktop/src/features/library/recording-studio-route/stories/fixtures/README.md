# Synthetic player fixture

`synthetic-player.webm` contains ten seconds of solid black video at one frame
per second and silent mono audio. It contains no captured media. Browser stories
serve it through Vite so failed placeholder URLs cannot overwrite deliberately
tested media failures or obstruct player controls.

Regenerate with installed FFmpeg:

```sh
ffmpeg -hide_banner -loglevel error -f lavfi -i color=c=black:s=160x90:r=1:d=10 -f lavfi -i anullsrc=r=48000:cl=mono -t 10 -c:v libvpx-vp9 -deadline realtime -cpu-used 8 -c:a libopus -b:a 8k -threads 1 -y apps/desktop/src/features/library/recording-studio-route/stories/fixtures/synthetic-player.webm
```
