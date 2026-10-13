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
for (let index = 0; index < 4; index++) {
  const path = join(directory, `speech-${index}.wav`);
  const speech = spawnSync(
    'espeak-ng',
    [
      '-v',
      'de',
      '-s',
      '150',
      '-w',
      path,
      'Dies ist eine vollständig synthetische deutsche Sprachaufnahme für einen lokalen Test.',
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
    'color=c=0x547266:size=320x180:rate=1/5:duration=50',
    ...clips.flatMap((path) => ['-i', path]),
    '-filter_complex',
    clips
      .map(
        (_, index) =>
          `[${index + 1}:a]adelay=${(1 + index * 11) * 1000}:all=1[a${index}]`,
      )
      .join(';') +
      ';[a0][a1][a2][a3]amix=inputs=4:normalize=0,apad=whole_dur=50[speech]',
    '-map',
    '0:v',
    '-map',
    '[speech]',
    '-t',
    '50',
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
    start + 50_000,
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
  db.prepare(
    'INSERT INTO recording_segments(id,recording_id,segment_index,file_path,started_at_ms,ended_at_ms,duration_ms,state,video_stream_index,desktop_audio_stream_index,microphone_audio_stream_index,created_at_ms,updated_at_ms,timeline_base_ms,timeline_basis_version) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)',
  ).run(
    segmentId,
    id,
    1,
    media,
    start,
    start + 50_000,
    50_000,
    'finalized',
    0,
    1,
    null,
    start,
    start,
    0,
    1,
  );
  db.prepare(
    'INSERT INTO recording_segment_videos(segment_id,track_id,stream_index,width,height,fps) VALUES(?,?,?,?,?,?)',
  ).run(segmentId, videoId, 0, 320, 180, 0.2);
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
      durationMs: 50_000,
      source: 'espeak-ng synthetic speech; actual native VAD',
    },
    null,
    2,
  ),
);
