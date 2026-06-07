import type {
  CaptureSource,
  RecorderSceneLayoutMode,
  RecorderSceneSource,
} from '@daydream/shared';

export type ResizeHandle = 'e' | 'n' | 'ne' | 'nw' | 's' | 'se' | 'sw' | 'w';
export type SceneLayoutMode = Exclude<RecorderSceneLayoutMode, 'custom'>;
export type SceneSourceRectResizeDimension = 'auto' | 'height' | 'width';

export interface SceneCanvasSize {
  height: number;
  width: number;
}

export interface SceneSourceDimensions {
  height: number;
  source: 'fallback' | 'logical' | 'physical';
  width: number;
}

export interface SceneSourceScale {
  scale: number | null;
  scaleX: number | null;
  scaleY: number | null;
  sourceHeight: number | null;
  sourceWidth: number | null;
}

export interface DuplicateCaptureSourceCandidate {
  existingSource: RecorderSceneSource;
  source: CaptureSource;
}

export interface CaptureSourceMergeClassification {
  duplicateCandidates: DuplicateCaptureSourceCandidate[];
  exactDuplicates: CaptureSource[];
  newSources: CaptureSource[];
}

export type DuplicateCaptureSourceResolution = 'refresh-existing' | 'skip';
