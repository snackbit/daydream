export interface RecoverLibraryRecordingInput {
  recordingId: string;
}
export interface RecordingRecoveryStreamCoverage {
  originalStreamIndex: number;
  recoveredStreamIndex: number;
  sourceStartOffsetMs: number;
  sourceEndOffsetMs: number;
}
export interface RecordingRecoveryCoverage {
  originalSegmentId: string;
  recoveredSegmentId: string;
  sourceStartOffsetMs: number;
  sourceEndOffsetMs: number;
  recoveredStartOffsetMs: number;
  recoveredEndOffsetMs: number;
  streams: RecordingRecoveryStreamCoverage[];
}
export interface RecordingRecoveryResult {
  operationId: string;
  originalRecordingId: string;
  recoveredRecordingId: string;
  status: 'recovered-partial';
  recoveredSegmentCount: number;
  skippedSegmentCount: number;
  coverage: RecordingRecoveryCoverage[];
}
