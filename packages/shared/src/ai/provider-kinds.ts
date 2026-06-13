export type AiProviderKind = 'local-whisper-cpp' | 'openai-compatible';

export type AiAssetProviderKind = AiProviderKind | 'local-tesseract-ocr';

export type AiVisionProviderKind = 'disabled' | 'openai-compatible';

export type AiOcrEngine = 'tesseract';

export type AiTranscriptionBackend = 'cpu' | 'cuda' | 'vulkan';

export type TranscriptSourceRole = 'desktop-audio' | 'microphone';

export type ActivitySourceKind =
  | 'frame-delta'
  | 'keyboard'
  | 'merged'
  | 'mouse';

export type ActivityIntervalState = 'active' | 'idle' | 'unknown';
