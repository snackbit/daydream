// Synthetic speech input only. VAD, requests and transcript results are produced by the app.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { DatabaseSync } from 'node:sqlite';

const root = process.env.DAYDREAM_E2E_DIR;
assert.ok(root?.startsWith('/tmp/daydream-native-e2e.'));
assert.equal(process.env.XDG_DATA_HOME, join(root, 'data'));
const id = randomUUID(),
  segmentId = randomUUID(),
  videoId = randomUUID();
const recordingsRoot = join(root, 'data/com.snackbit.daydream/recordings');
const directory = join(recordingsRoot, id);
await mkdir(directory, { recursive: true });
const clips = [];
const speechStarts = [1, 10, 26, 35, 43, 60, 68, 87, 95, 113, 121];
for (let index = 0; index < 11; index++) {
  const path = join(directory, `speech-${index}.wav`);
  const speech = spawnSync(
    'espeak-ng',
    [
      '-v',
      'de',
      '-s',
      String(140 + index * 3),
      '-w',
      path,
      'Dies ist eine synthetische Aufnahme.',
    ],
    { encoding: 'utf8' },
  );
  assert.equal(
    speech.status,
    0,
    'espeak-ng must generate isolated synthetic speech',
  );
  clips.push(path);
}
const media = join(directory, 'synthetic-language.mkv');
const encoder = spawnSync(
  process.env.DAYDREAM_FFMPEG_PATH,
  [
    '-hide_banner',
    '-loglevel',
    'error',
    '-y',
    '-f',
    'lavfi',
    '-i',
    'color=c=0x547266:size=320x180:rate=1/5:duration=128',
    ...clips.flatMap((path) => ['-i', path]),
    '-filter_complex',
    clips
      .map(
        (_, index) =>
          `[${index + 1}:a]adelay=${speechStarts[index] * 1000}:all=1[a${index}]`,
      )
      .join(';') +
      `;${clips.map((_, index) => `[a${index}]`).join('')}amix=inputs=${clips.length}:normalize=0,apad=whole_dur=128[speech]`,
    '-map',
    '0:v',
    '-map',
    '[speech]',
    '-t',
    '128',
    '-c:v',
    'libx264',
    '-threads',
    '1',
    '-preset',
    'ultrafast',
    '-c:a',
    'libopus',
    media,
  ],
  { encoding: 'utf8' },
);
assert.equal(encoder.status, 0, encoder.stderr);
// Two source runs, separated by a wall-clock gap. The first two tentative
// detections must be repaired from their own earlier segment and chunks.
const sources = [
  { id: segmentId, from: 0, duration: 22, base: 0 },
  { id: randomUUID(), from: 22, duration: 106, base: 27_000 },
];
for (const [index, source] of sources.entries()) {
  source.path = join(directory, `segment-${index}.mkv`);
  const split = spawnSync(
    process.env.DAYDREAM_FFMPEG_PATH,
    [
      '-hide_banner',
      '-loglevel',
      'error',
      '-y',
      '-ss',
      String(source.from),
      '-i',
      media,
      '-t',
      String(source.duration),
      '-map',
      '0',
      '-c:v',
      'libx264',
      '-threads',
      '1',
      '-preset',
      'ultrafast',
      '-c:a',
      'libopus',
      source.path,
    ],
    { encoding: 'utf8' },
  );
  assert.equal(split.status, 0, split.stderr);
}
const date = new Date(Date.now() - 2 * 86_400_000).toISOString().slice(0, 10),
  start = Date.parse(`${date}T08:00:00Z`);
const db = new DatabaseSync(
  join(root, 'data/com.snackbit.daydream/daydream.sqlite3'),
);
db.exec('PRAGMA foreign_keys=ON; PRAGMA busy_timeout=10000; BEGIN');
try {
  assert.ok(
    db
      .prepare(
        "SELECT 1 FROM sqlite_master WHERE name='processing_speech_languages'",
      )
      .get(),
    'bootstrap schema 37 first',
  );
  db.prepare(
    'INSERT INTO recordings(id,created_at_ms,started_at_ms,ended_at_ms,status,default_fps,quality_label,storage_dir,output_root,title) VALUES(?,?,?,?,?,?,?,?,?,?)',
  ).run(
    id,
    start,
    start,
    start + 133_000,
    'completed',
    0.2,
    'balanced-ocr',
    directory,
    recordingsRoot,
    'Synthetic language correction',
  );
  db.prepare(
    'INSERT INTO recording_tracks(id,recording_id,kind,label,source_id,created_at_ms) VALUES(?,?,?,?,?,?)',
  ).run(videoId, id, 'screen', 'Synthetic screen', 'synthetic-language', start);
  db.prepare(
    'INSERT INTO recording_video_sources(track_id,recording_id,source_id,label,width,height,config_json) VALUES(?,?,?,?,?,?,?)',
  ).run(videoId, id, 'synthetic-language', 'Synthetic screen', 320, 180, '{}');
  for (const [index, source] of sources.entries()) {
    db.prepare(
      'INSERT INTO recording_segments(id,recording_id,segment_index,file_path,started_at_ms,ended_at_ms,duration_ms,state,video_stream_index,desktop_audio_stream_index,microphone_audio_stream_index,created_at_ms,updated_at_ms,timeline_base_ms,timeline_basis_version) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)',
    ).run(
      source.id,
      id,
      index + 1,
      source.path,
      start + source.base,
      start + source.base + source.duration * 1000,
      source.duration * 1000,
      'finalized',
      0,
      1,
      null,
      start,
      start,
      source.base,
      1,
    );
    db.prepare(
      'INSERT INTO recording_segment_videos(segment_id,track_id,stream_index,width,height,fps) VALUES(?,?,?,?,?,?)',
    ).run(source.id, videoId, 0, 320, 180, 0.2);
  }
  db.exec('COMMIT');
} catch (error) {
  db.exec('ROLLBACK');
  throw error;
} finally {
  db.close();
}
await writeFile(
  join(root, 'language-fixture.json'),
  JSON.stringify(
    {
      id,
      date,
      durationMs: 133_000,
      source: 'espeak-ng synthetic speech; actual native VAD',
    },
    null,
    2,
  ),
);
