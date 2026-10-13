export interface RecordingAiResultInput {
  startOffsetMs?: number;
  endOffsetMs?: number;
  bucketMs?: number;
  reviewStartOffsetMs?: number;
  reviewEndOffsetMs?: number;
  recordingId: string;
  includeOcrRegions?: boolean;
}
