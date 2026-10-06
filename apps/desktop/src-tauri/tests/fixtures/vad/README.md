# Synthetic speech-detection fixtures

`reply.wav` uses the same eSpeak NG voice and rate to say “Yes.” It covers a
brief reply and speech spanning a processing boundary, without recorded audio.

These fixtures contain no captured audio. `typing.wav` contains deterministic
3 ms, 3.3 kHz taps every 130 ms, generated with FFmpeg's `aevalsrc` at 16 kHz.
`speech.wav` is eSpeak NG's `en-us` voice at 155 words/minute saying:
"Yes. Please save this synthetic test." It was resampled with FFmpeg to mono
16 kHz signed PCM16. Tests derive quiet speech and speech/noise mixtures locally.

The generated audio is provided under the repository's license. eSpeak NG and
FFmpeg are fixture-generation tools, not additional application dependencies.
