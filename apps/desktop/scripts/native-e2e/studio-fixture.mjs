// Saved-result load fixture, not a model-quality test. All media and text are synthetic.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { mkdir, link, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { DatabaseSync } from 'node:sqlite';

const root = process.env.DAYDREAM_E2E_DIR;
assert.ok(root?.startsWith('/tmp/daydream-native-e2e.'));
assert.equal(process.env.XDG_DATA_HOME, join(root, 'data'));
const db = new DatabaseSync(
  join(root, 'data/com.snackbit.daydream/daydream.sqlite3'),
);
db.exec('PRAGMA foreign_keys=ON; PRAGMA busy_timeout=10000');
assert.ok(
  db
    .prepare("SELECT 1 FROM sqlite_master WHERE name='studio_visual_samples'")
    .get(),
  'run initial phase with current native build first',
);
const sample = join(root, 'synthetic-studio.mkv');
const encode = spawnSync(
  process.env.DAYDREAM_FFMPEG_PATH,
  [
    '-hide_banner',
    '-loglevel',
    'error',
    '-y',
    '-f',
    'lavfi',
    '-i',
    'testsrc2=size=640x360:rate=1/5:duration=30',
    '-f',
    'lavfi',
    '-i',
    'color=c=0x547266:size=640x360:rate=1/5:duration=30',
    '-f',
    'lavfi',
    '-i',
    'sine=frequency=220:sample_rate=48000:duration=30',
    '-f',
    'lavfi',
    '-i',
    'sine=frequency=330:sample_rate=48000:duration=30',
    '-map',
    '0:v',
    '-map',
    '1:v',
    '-map',
    '2:a',
    '-map',
    '3:a',
    '-c:v',
    'libx264',
    '-threads',
    '1',
    '-preset',
    'ultrafast',
    '-c:a',
    'libopus',
    sample,
  ],
  { encoding: 'utf8' },
);
assert.equal(encode.status, 0, encode.stderr);
const date = new Date(Date.now() - 86_400_000).toISOString().slice(0, 10);
const origin = Date.parse(`${date}T08:00:00Z`);
const recordingMs = 7_200_000;
const recordingsRoot = join(root, 'data/com.snackbit.daydream/recordings');
const recordings = [];
const insert = (table, columns) =>
  db.prepare(
    `INSERT INTO ${table}(${columns.join(',')}) VALUES(${columns.map(() => '?').join(',')})`,
  );
const recording = insert('recordings', [
  'id',
  'created_at_ms',
  'started_at_ms',
  'ended_at_ms',
  'status',
  'default_fps',
  'quality_label',
  'storage_dir',
  'output_root',
  'title',
]);
const track = insert('recording_tracks', [
  'id',
  'recording_id',
  'kind',
  'label',
  'source_id',
  'created_at_ms',
]);
const video = insert('recording_video_sources', [
  'track_id',
  'recording_id',
  'source_id',
  'label',
  'width',
  'height',
  'config_json',
]);
const segment = insert('recording_segments', [
  'id',
  'recording_id',
  'segment_index',
  'file_path',
  'started_at_ms',
  'ended_at_ms',
  'duration_ms',
  'state',
  'video_stream_index',
  'desktop_audio_stream_index',
  'microphone_audio_stream_index',
  'created_at_ms',
  'updated_at_ms',
  'timeline_base_ms',
  'timeline_basis_version',
]);
const segmentVideo = insert('recording_segment_videos', [
  'segment_id',
  'track_id',
  'stream_index',
  'width',
  'height',
  'fps',
]);
const job = insert('processing_jobs', [
  'id',
  'recording_id',
  'kind',
  'status',
  'created_at_ms',
  'updated_at_ms',
]);
const chunk = insert('processing_chunks', [
  'id',
  'job_id',
  'recording_id',
  'source_kind',
  'segment_id',
  'track_id',
  'start_offset_ms',
  'end_offset_ms',
  'status',
  'progress',
  'created_at_ms',
  'updated_at_ms',
]);
const speech = insert('transcript_segments', [
  'id',
  'recording_id',
  'segment_id',
  'track_id',
  'source_role',
  'start_offset_ms',
  'end_offset_ms',
  'provider',
  'text',
  'job_id',
  'chunk_id',
  'created_at_ms',
]);
const word = insert('transcript_words', [
  'id',
  'transcript_segment_id',
  'start_offset_ms',
  'end_offset_ms',
  'text',
]);
const activity = insert('frame_activity_intervals', [
  'id',
  'recording_id',
  'source_kind',
  'state',
  'start_offset_ms',
  'end_offset_ms',
  'job_id',
  'chunk_id',
]);
const observation = insert('vlm_observations', [
  'id',
  'recording_id',
  'frame_timestamp_ms',
  'source_frame_ref',
  'region_x',
  'region_y',
  'region_width',
  'region_height',
  'text',
  'provider',
  'prompt_version',
  'job_id',
  'chunk_id',
]);
for (let r = 0; r < (process.argv[2] === 'inference' ? 0 : 4); r++) {
  const id = randomUUID(),
    start = origin + r * recordingMs,
    directory = join(recordingsRoot, id);
  await mkdir(directory, { recursive: true });
  const fragments = [];
  for (let s = 0; s < 240; s++) {
    const path = join(directory, `synthetic-${s}.mkv`);
    await link(sample, path);
    fragments.push({ id: randomUUID(), path });
  }
  db.exec('BEGIN');
  try {
    recording.run(
      id,
      start,
      start,
      start + recordingMs,
      'completed',
      0.2,
      'balanced-ocr',
      directory,
      recordingsRoot,
      `Synthetic Studio ${r + 1}`,
    );
    const tracks = Array.from({ length: 4 }, () => randomUUID());
    for (const [i, label] of [
      'Screen 1',
      'Screen 2',
      'Desktop audio',
      'Microphone',
    ].entries()) {
      track.run(
        tracks[i],
        id,
        i < 2 ? 'screen' : i === 2 ? 'desktop-audio' : 'microphone',
        label,
        i < 2 ? `synthetic-screen-${i}` : null,
        start,
      );
      if (i < 2)
        video.run(
          tracks[i],
          id,
          `synthetic-screen-${i}`,
          label,
          640,
          360,
          '{}',
        );
    }
    const jobId = randomUUID();
    job.run(jobId, id, 'ai-processing', 'succeeded', start, start);
    const owners = [];
    for (const [s, f] of fragments.entries()) {
      const base = s * 30_000;
      segment.run(
        f.id,
        id,
        s + 1,
        f.path,
        start + base,
        start + base + 30_000,
        30_000,
        'finalized',
        0,
        2,
        3,
        start,
        start,
        base,
        1,
      );
      for (let v = 0; v < 2; v++)
        segmentVideo.run(f.id, tracks[v], v, 640, 360, 0.2);
      const owner = randomUUID();
      owners.push(owner);
      chunk.run(
        owner,
        jobId,
        id,
        'audio',
        f.id,
        tracks[2],
        base,
        base + 30_000,
        'succeeded',
        1,
        start,
        start,
      );
      for (let v = 0; v < 2; v++) {
        const visualOwner = randomUUID();
        chunk.run(
          visualOwner,
          jobId,
          id,
          'vlm',
          f.id,
          tracks[v],
          base,
          base + 30_000,
          'succeeded',
          1,
          start,
          start,
        );
        observation.run(
          randomUUID(),
          id,
          base,
          `segment:${f.id}:stream:${tracks[v]}:offset:${base}:frame:0`,
          0,
          0,
          640,
          360,
          `Synthetic screen change ${s + 1}`,
          'openai-compatible',
          'legacy-fixture',
          jobId,
          visualOwner,
        );
      }
    }
    for (let i = 0; i < 12_500; i++) {
      const from = Math.floor((i * recordingMs) / 12_500),
        to = from + 400,
        s = Math.floor(from / 30_000),
        utterance = randomUUID();
      speech.run(
        utterance,
        id,
        fragments[s].id,
        tracks[2],
        'desktop-audio',
        from,
        to,
        'local-whisper-cpp',
        'Synthetic spoken entry',
        jobId,
        owners[s],
        start,
      );
      for (const [w, text] of ['Synthetic', 'spoken', 'entry'].entries())
        word.run(
          randomUUID(),
          utterance,
          from + w * 100,
          from + (w + 1) * 100,
          text,
        );
    }
    for (let i = 0; i < 25_000; i++) {
      const from = Math.floor((i * recordingMs) / 25_000),
        owner = owners[Math.floor(from / 30_000)];
      activity.run(
        randomUUID(),
        id,
        i % 2 ? 'mouse' : 'keyboard',
        'active',
        from,
        from + 100,
        jobId,
        owner,
      );
    }
    db.exec('COMMIT');
    recordings.push(id);
  } catch (error) {
    db.exec('ROLLBACK');
    throw error;
  }
}
const settings = db.prepare(
  'INSERT INTO app_settings(key,value_json,created_at_ms,updated_at_ms) VALUES(?,?,?,?) ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json,updated_at_ms=MAX(app_settings.created_at_ms,excluded.updated_at_ms)',
);
const visionId = randomUUID(),
  visionStart = origin - 86_400_000;
const visionDirectory = join(recordingsRoot, visionId);
await mkdir(visionDirectory, { recursive: true });
const visionPath = join(visionDirectory, 'synthetic.mkv');
await link(sample, visionPath);
recording.run(
  visionId,
  visionStart,
  visionStart,
  visionStart + 30_000,
  'completed',
  0.2,
  'balanced-ocr',
  visionDirectory,
  recordingsRoot,
  'Synthetic screen inference',
);
const visionSegment = randomUUID();
segment.run(
  visionSegment,
  visionId,
  1,
  visionPath,
  visionStart,
  visionStart + 30_000,
  30_000,
  'finalized',
  0,
  null,
  null,
  visionStart,
  visionStart,
  0,
  1,
);
for (let index = 0; index < 2; index++) {
  const source = randomUUID();
  track.run(
    source,
    visionId,
    'screen',
    `Synthetic source ${index + 1}`,
    `synthetic-${index}`,
    visionStart,
  );
  video.run(
    source,
    visionId,
    `synthetic-${index}`,
    `Synthetic source ${index + 1}`,
    640,
    360,
    '{}',
  );
  segmentVideo.run(visionSegment, source, index, 640, 360, 0.2);
}
for (const [key, value] of [
  ['ai.ocr.enabled', false],
  ['ai.vision.observationsEnabled', false],
  ['ai.processing.autoStartAfterRecording', false],
  ['ai.transcription.provider', 'local-whisper-cpp'],
])
  settings.run(key, JSON.stringify(value), origin, origin);
assert.deepEqual(db.prepare('PRAGMA foreign_key_check').all(), []);
db.close();
await writeFile(
  join(root, 'studio-fixture.json'),
  JSON.stringify(
    {
      date,
      recordings,
      recordingsRoot,
      visionId,
      durationMs: 28_800_000,
      fragments: 960,
      sources: 4,
      speech: 50_000,
      words: 150_000,
      activity: 100_000,
      observations: 1920,
    },
    null,
    2,
  ),
);
