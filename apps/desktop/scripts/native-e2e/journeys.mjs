import assert from 'node:assert/strict';
import {
  copyFile,
  readFile,
  readdir,
  rename,
  stat,
  writeFile,
} from 'node:fs/promises';
import { setTimeout as delay } from 'node:timers/promises';
import { chromium } from 'playwright';
import { checkDayReview } from './day-review.mjs';
import { writeNativeFailureDiagnostics } from './failure-diagnostics.mjs';
import { connectInspector } from './inspector.mjs';
import { checkLanguageContinuity } from './language.mjs';
import { checkLibraryDays } from './library.mjs';
import { checkLongPlayback, longPlayback } from './long-playback.mjs';
import { checkModelSettings } from './model-settings.mjs';
import { createNativePointer } from './pointer.mjs';
import { checkSearchKeyboard } from './search-keyboard.mjs';
import { checkStudioControls } from './studio-controls.mjs';
import { checkStudioInference } from './studio-inference.mjs';
import { checkStudioLoad } from './studio-load.mjs';
import { checkPreparedStudioDay } from './studio-prepared-day.mjs';
import { checkPreparedStudioTiming } from './studio-ready-timing.mjs';
import { checkSeekedStudioTranscripts } from './transcripts/studio-transcript-groups.mjs';
import { connectNativeKeyboard } from './webdriver.mjs';

const phase = process.argv[2] ?? 'initial';
const artifacts = process.env.DAYDREAM_E2E_DIR;
if (!artifacts?.startsWith('/tmp/daydream-native-e2e.')) {
  throw new Error('Use run.sh to isolate synthetic application data');
}
const stories = [];
const browser = await chromium.launch({ headless: true });
let native;
let viewer;
let failure;
let keyboard;
try {
  viewer = await browser.newPage({
    viewport: { width: 1440, height: 1000 },
  });
  await viewer.goto('http://127.0.0.1:39188', {
    waitUntil: 'domcontentloaded',
  });
  if (phase === 'keyboard') {
    keyboard = await connectNativeKeyboard();
    native = keyboard;
  } else native = await connectInspector();
  await native.wait(
    'document.querySelector("nav a[aria-label=Settings]") !== null',
    'application navigation',
  );
  const distIndex = await readFile(
    new URL('../../dist/index.html', import.meta.url),
    'utf8',
  );
  const expectedAssets = [
    ...distIndex.matchAll(/(?:src|href)="([^"\s]+\.(?:js|css))"/g),
  ]
    .map((match) => match[1])
    .sort();
  const actualAssets = JSON.parse(
    await native.evaluate(`JSON.stringify([
    ...document.querySelectorAll('script[src],link[href]')
  ].map(element => element.getAttribute(element.tagName === 'SCRIPT' ? 'src' : 'href')).filter(value => value.endsWith('.js') || value.endsWith('.css')).sort())`),
  ).map((value) => new URL(value, 'tauri://localhost').pathname);
  const builtAssets = await readdir(
    new URL('../../dist/assets/', import.meta.url),
  );
  assert.ok(
    expectedAssets.length > 0,
    'production build must contain hashed assets',
  );
  await writeFile(
    `${artifacts}/${phase}-assets.json`,
    JSON.stringify({ expectedAssets, actualAssets }, null, 2),
  );
  for (const asset of expectedAssets) {
    assert.ok(
      actualAssets.includes(asset),
      `Native startup asset must match current frontend build: ${asset}`,
    );
  }
  for (const asset of actualAssets) {
    assert.ok(
      builtAssets.includes(asset.split('/').at(-1)),
      `Native lazy asset must exist in current frontend build: ${asset}`,
    );
  }
  stories.push('NATIVE-CURRENT-FRONTEND-ASSET-IDENTITY');
  assert.equal(
    await native.evaluate('devicePixelRatio'),
    1,
    'isolated Broadway display DPI',
  );
  // GTK draws titlebar and shadow inside Broadway's canvas. Locate the actual
  // WebView origin from a temporary rendered marker instead of guessing chrome
  // dimensions. The marker never handles input and is removed before any story.
  await native.evaluate(`(() => {
    const marker = document.createElement('div');
    marker.id = 'native-e2e-origin';
    marker.style.cssText = 'position:fixed;left:0;top:0;width:8px;height:8px;background:rgb(13,253,71);z-index:2147483647;pointer-events:none';
    document.body.appendChild(marker);
    return true;
  })()`);
  let origin;
  try {
    const handle = await viewer.waitForFunction(
      () => {
        const canvases = [...document.querySelectorAll('canvas')];
        for (const [index, canvas] of canvases.entries()) {
          const pixels = canvas
            .getContext('2d')
            .getImageData(0, 0, canvas.width, canvas.height).data;
          for (let offset = 0; offset < pixels.length; offset += 4) {
            if (
              pixels[offset] === 13 &&
              pixels[offset + 1] === 253 &&
              pixels[offset + 2] === 71
            ) {
              return {
                index,
                x: (offset / 4) % canvas.width,
                y: Math.floor(offset / 4 / canvas.width),
                width: canvas.width,
                height: canvas.height,
              };
            }
          }
        }
        return false;
      },
      null,
      { timeout: 8000 },
    );
    origin = await handle.jsonValue();
    await writeFile(
      `${artifacts}/${phase}-pointer-origin.json`,
      JSON.stringify(origin, null, 2),
    );
  } finally {
    await native.evaluate(
      'document.getElementById("native-e2e-origin")?.remove()',
    );
  }

  const click =
    phase === 'keyboard'
      ? keyboard.click
      : createNativePointer({ native, viewer, origin });
  async function button(text) {
    const selector = await native.evaluate(`(() => {
      const candidates = [...document.querySelectorAll('button')].filter(e => e.textContent.trim() === ${JSON.stringify(text)});
      const button = candidates.find(e => e.getBoundingClientRect().width > 0);
      if (!button) throw new Error('Missing button');
      button.setAttribute('data-native-e2e-button','active');
      return '[data-native-e2e-button="active"]';
    })()`);
    try {
      await click(selector);
    } finally {
      await native.evaluate(
        `document.querySelector(${JSON.stringify(selector)})?.removeAttribute('data-native-e2e-button')`,
      );
    }
  }
  async function navigate(label, path) {
    await click(`nav a[aria-label="${label}"]`);
    await native.wait(
      `location.pathname === ${JSON.stringify(path)}`,
      `navigate ${label}`,
    );
  }
  async function checkbox(label) {
    const selector = await native.evaluate(`(() => {
      const switchControl = [...document.querySelectorAll('[role=switch]')].find(e => e.getAttribute('aria-label') === ${JSON.stringify(label)});
      if (switchControl) return '[role="switch"][aria-label=' + JSON.stringify(${JSON.stringify(label)}) + ']';
      const label = [...document.querySelectorAll('label')].find(e => e.textContent.includes(${JSON.stringify(label)}));
      if (!label) throw new Error('Missing checkbox label');
      return '#' + CSS.escape(label.htmlFor);
    })()`);
    return selector;
  }

  const studioJourney = { native, navigate, click, viewer, artifacts };
  if (phase === 'bootstrap') {
    const settings = (await native.invoke('getAppSettings')).settings;
    assert.equal(settings.ai.vision.provider, 'disabled');
    stories.push('NATIVE-STUDIO-ISOLATED-SCHEMA-BOOTSTRAP');
  } else if (phase === 'keyboard') {
    await checkSearchKeyboard({
      native,
      viewer,
      keyboard,
      navigate,
      click,
      artifacts,
      phase,
    });
    stories.push('NATIVE-SEARCH-TYPE-SUBMIT-EMPTY-RESULT');
  } else if (phase === 'inference') {
    const fixture = JSON.parse(
      await readFile(`${artifacts}/studio-fixture.json`, 'utf8'),
    );
    await checkStudioInference({
      native,
      click,
      button,
      navigate,
      viewer,
      recordingId: fixture.visionId,
      artifacts,
      afterBaseline: async () => {
        await checkDayReview({
          native,
          click,
          button,
          navigate,
          viewer,
          recordingId: fixture.visionId,
          artifacts,
        });
        stories.push('NATIVE-DAY-REVIEW-GENERATION-FAILURE-RETRY-PUBLICATION');
        await click('[data-slot="sheet-content"] button:has(.lucide-x)');
        await native.wait(
          `document.querySelector('[data-slot="sheet-content"]') === null`,
          'review closes before recovery navigation',
        );
      },
    });
    stories.push('NATIVE-STUDIO-SHARED-INFERENCE-PARTIAL-RETRY-CONTEXT');
    stories.push('NATIVE-PROCESSING-PIPELINE-SCOPED-OVERVIEW-LIVE-DEPENDENCY');
    stories.push('NATIVE-LOCAL-VISUAL-RECOVERY-INDEPENDENT-DESCRIPTIONS');
  } else if (phase === 'language') {
    await checkLanguageContinuity({
      native,
      artifacts,
      navigate,
      click,
      viewer,
    });
    stories.push('NATIVE-AUTO-LANGUAGE-CONSENSUS-RETROACTIVE-CORRECTION');
  } else if (phase === 'studio-transcripts') {
    await checkSeekedStudioTranscripts(studioJourney);
    stories.push('NATIVE-STUDIO-SEEKED-TRANSCRIPT-GROUPS');
  } else if (phase === 'studio-ready') {
    stories.push(await checkPreparedStudioDay(studioJourney));
  } else if (phase === 'studio') {
    const diagnostic = await checkStudioLoad({ ...studioJourney, button });
    stories.push(
      diagnostic === 'diagnostic'
        ? 'NATIVE-STUDIO-LAYOUT-DIAGNOSTIC'
        : 'NATIVE-STUDIO-EIGHT-HOUR-LOAD-REFRESH-CACHE-RESPONSIVENESS',
    );
  } else {
    await navigate('Settings', '/settings');
    await native.wait(
      'document.querySelector("[role=switch]") !== null',
      'settings loaded',
    );
    const expert = await checkbox('Enable expert mode');
    const initial = (await native.invoke('getAppSettings')).settings;
    if (phase === 'restart') {
      assert.equal(
        initial.expert.enabled,
        true,
        'UI save must survive native process restart',
      );
      assert.equal(
        await native.evaluate(
          `document.querySelector(${JSON.stringify(expert)}).getAttribute('aria-checked')`,
        ),
        'true',
      );
      stories.push('NATIVE-SETTINGS-RESTART');
      const preservedRecordings = await native.invoke('listLibraryRecordings');
      assert.ok(
        preservedRecordings.length > 0,
        'synthetic recordings survive restart',
      );
      await click('[aria-label="Color appearance"]');
      await native.wait(
        'document.querySelector("[role=option]") !== null',
        'appearance choices',
      );
      await native.evaluate(
        `(() => { const option = [...document.querySelectorAll('[role=option]')].find(element => element.textContent.trim() === 'Dark'); if (!option) throw new Error('Missing dark appearance option'); option.setAttribute('data-native-e2e-dark','active'); return true; })()`,
      );
      await click('[data-native-e2e-dark="active"]');
      await native.wait(
        'document.documentElement.dataset.theme === "dark"',
        'dark appearance saved',
      );
      await button('Reset preferences');
      await native.wait(
        'document.querySelector("[role=alertdialog]") !== null',
        'reset impact review',
      );
      await button('Cancel');
      await native.wait(
        'document.querySelector("[role=alertdialog]") === null',
        'reset impact review cancelled',
      );
      assert.equal(
        (await native.invoke('getAppSettings')).settings.expert.enabled,
        true,
      );
      assert.equal(
        await native.evaluate('document.documentElement.dataset.theme'),
        'dark',
      );
      await button('Reset preferences');
      await native.wait(
        'document.querySelector("[role=alertdialog]") !== null',
        'reset impact review reopened',
      );
      await button('Continue');
      assert.equal(
        (await native.invoke('getAppSettings')).settings.expert.enabled,
        true,
        'impact review alone does not reset preferences',
      );
      await button('Confirm');
      await native.wait(
        `document.querySelector('[aria-label="Enable expert mode"]')?.getAttribute('aria-checked') === 'false'`,
        'reset defaults',
      );
      assert.equal(
        (await native.invoke('getAppSettings')).settings.expert.enabled,
        false,
      );
      await native.wait(
        'document.documentElement.dataset.theme === "light" && localStorage.getItem("daydream.appearance") === null',
        'reset appearance persisted before native acknowledgement',
      );
      assert.equal(
        await native.invoke('getMaintenanceStatus'),
        null,
        'native cleanup and theme acknowledgement completed',
      );
      const afterResetRecordings = await native.invoke('listLibraryRecordings');
      assert.deepEqual(
        afterResetRecordings.map((recording) => recording.id).sort(),
        preservedRecordings.map((recording) => recording.id).sort(),
        'preference reset preserves the complete library',
      );
      for (const recording of afterResetRecordings) {
        const detail = await native.invoke('getLibraryRecording', {
          recordingId: recording.id,
        });
        const before = preservedRecordings.find(
          (before) => before.id === recording.id,
        );
        assert.ok(before, 'reset retains the original recording');
        assert.equal(
          detail.playableSegmentCount,
          before.playableSegmentCount,
          'reset preserves every playable fragment',
        );
        for (const segment of detail.segments) {
          assert.equal(segment.fileExists, true);
          assert.ok(segment.filePath.startsWith(`${artifacts}/data/`));
          assert.ok((await stat(segment.filePath)).size > 0);
        }
      }
      stories.push('NATIVE-SETTINGS-RESET-CANCEL-CONFIRM');
    } else {
      assert.equal(initial.ai.transcription.provider, 'local-whisper-cpp');
      assert.notEqual(initial.ai.vision.provider, 'openai-compatible');
      assert.notEqual(
        initial.ai.recall.embedding.provider,
        'openai-compatible',
      );
      assert.equal(initial.ai.recall.generation.enabled, false);
      assert.equal(initial.expert.enabled, false);
      stories.push('NATIVE-LOCAL-CONSENT-DEFAULTS');
      await click(expert);
      await native.wait(
        `document.body.textContent.includes('Saved') && document.querySelector('[aria-label="Enable expert mode"]')?.getAttribute('aria-checked') === 'true'`,
        'autosaved settings',
      );
      assert.equal(
        (await native.invoke('getAppSettings')).settings.expert.enabled,
        true,
      );
      stories.push('NATIVE-SETTINGS-SAVE');
      await checkModelSettings({ native, button, checkbox, click });
      stories.push('NATIVE-MODEL-FAILURE-LOGGING-RETRY');
      stories.push('NATIVE-MODEL-CAPABILITIES-SAVE-WITHOUT-REMOTE-CONSENT');
      await click(await checkbox('Enable expert mode'));
      await navigate('Library', '/library');
      await native.wait(
        'document.querySelector("[role=dialog]") === null',
        'autosave allows navigation',
      );
      assert.equal(
        (await native.invoke('getAppSettings')).settings.expert.enabled,
        false,
      );
      await navigate('Settings', '/settings');
      await click(await checkbox('Enable expert mode'));
      await navigate('Library', '/library');
      assert.equal(
        (await native.invoke('getAppSettings')).settings.expert.enabled,
        true,
      );
      stories.push('NATIVE-SETTINGS-AUTOSAVE-NAVIGATION');
      await navigate('Search', '/search');
      await native.wait(
        'document.querySelector("main input") !== null',
        'search ready',
      );
      await navigate('Processing', '/processing');
      await native.wait(
        'document.querySelector("main h1") !== null',
        'processing ready',
      );
      await navigate('Library', '/library');
      await native.wait(
        'document.body.textContent.includes("Your next day")',
        'empty library',
      );
      stories.push('NATIVE-NAVIGATION-EMPTY-LIBRARY-SEARCH-PROCESSING');
      // Fixture setup uses the public native settings boundary. Disable ambient
      // window providers and automatic processing before synthetic recording.
      const captureSettings = (await native.invoke('getAppSettings')).settings;
      captureSettings.ai.activity.windows.enabled = false;
      captureSettings.ai.processing.timing = 'manual';
      captureSettings.ai.processing.autoStartAfterRecording = false;
      captureSettings.ai.ocr.enabled = false;
      await native.invoke('saveAppSettings', {
        input: { settings: captureSettings },
      });
      await navigate('Capture', '/capture');
      await click('button[aria-label="Add monitor sources"]');
      await native.wait(
        `[...document.querySelectorAll('button')].some(element => element.textContent.trim() === 'Save' && !element.disabled)`,
        'source selection ready to save',
      );
      await button('Save');
      await native.wait(
        'document.body.textContent.includes("Source setup saved")',
        'synthetic source saved',
      );
      const scene = await native.invoke('getRecorderScene');
      assert.ok(scene.sources.length > 0);
      assert.ok(
        scene.sources.every((source) => source.sourceId.startsWith('fake-')),
      );
      assert.equal(scene.desktopAudioSourceId, null);
      assert.equal(scene.microphoneAudioSourceId, null);
      // Replace only the isolated package copy, as a build would replace a live
      // executable. Start and Resume must still launch the parent's media worker.
      const executable = `${artifacts}/package/daydream-desktop`;
      const originalInode = (await stat(executable)).ino;
      await copyFile(executable, `${executable}.replacement`);
      await rename(`${executable}.replacement`, executable);
      assert.notEqual((await stat(executable)).ino, originalInode);
      await button('Start');
      await native.wait(
        'document.body.textContent.includes("Status: recording")',
        'recording',
      );
      assert.equal(
        (await native.invoke('getRecorderStatus')).state,
        'recording',
      );
      let elapsed = 0;
      // Leave enough source time for measured 2x playback and a later resume.
      const captureDurationMs = longPlayback ? 103_000 : 8000;
      for (
        let attempt = 0;
        attempt < (longPlayback ? 1200 : 120) && elapsed < captureDurationMs;
        attempt += 1
      ) {
        elapsed =
          (await native.invoke('getRecorderRecordingStats'))
            .recordedDurationMs ?? 0;
        await delay(100);
      }
      assert.ok(
        elapsed >= captureDurationMs,
        'native writer must capture a playable segment',
      );
      await button('Pause');
      await native.wait(
        'document.body.textContent.includes("Status: paused")',
        'paused',
      );
      const pausedStatus = await native.invoke('getRecorderStatus');
      await writeFile(
        `${artifacts}/${phase}-paused-native.json`,
        JSON.stringify(pausedStatus, null, 2),
      );
      await writeFile(
        `${artifacts}/${phase}-source-scene.json`,
        JSON.stringify(scene, null, 2),
      );
      assert.equal(pausedStatus.state, 'paused');
      await button('Resume');
      await native.wait(
        'document.body.textContent.includes("Status: recording")',
        'resumed',
      );
      await button('Stop');
      await native.wait(
        'window.location.pathname.startsWith("/day/") && document.querySelector("video") !== null',
        'durably saved recording opened in Studio',
      );
      await native.wait(
        '!document.body.textContent.includes("Saving recording")',
        'native stop finalization completed',
      );
      const stoppedStatus = await native.invoke('getRecorderStatus');
      assert.equal(stoppedStatus.state, 'idle');
      assert.equal(stoppedStatus.finalization, null);
      stories.push('NATIVE-SYNTHETIC-CAPTURE-SAVE-START-PAUSE-RESUME-STOP');
      stories.push('NATIVE-CAPTURE-AFTER-EXECUTABLE-REPLACEMENT');
      const recordings = await native.invoke('listLibraryRecordings');
      assert.equal(recordings.length, 1);
      assert.equal(recordings[0].availability, 'available');
      const detail = await native.invoke('getLibraryRecording', {
        recordingId: recordings[0].id,
      });
      assert.ok(detail.playableSegmentCount >= 2);
      for (const segment of detail.segments) {
        assert.equal(segment.fileExists, true);
        assert.ok(segment.filePath.startsWith(`${artifacts}/data/`));
        assert.ok((await stat(segment.filePath)).size > 0);
      }
      await checkLibraryDays({ native, navigate, viewer, artifacts });
      stories.push('NATIVE-LIBRARY-METADATA-DAYS');
      const dayOpeningStarted = performance.now();
      await click('a[href^="/day/"]');
      await native.wait(
        'document.querySelector("video") !== null',
        'native Studio video',
      );
      await native.wait(
        'document.querySelector("video").readyState >= 2',
        'native decoded video frame',
      );
      await writeFile(
        `${artifacts}/initial-day-opening-timings.json`,
        JSON.stringify(
          { navigationToDecodedFrameMs: performance.now() - dayOpeningStarted },
          null,
          2,
        ),
      );
      assert.ok(
        await native.evaluate('document.querySelector("video").videoWidth > 0'),
      );
      stories.push('NATIVE-RECORDED-MEDIA-LIBRARY-STUDIO');
      await checkPreparedStudioTiming({
        native,
        navigate,
        click,
        artifacts,
        recordingId: recordings[0].id,
      });
      await viewer.screenshot({ path: `${artifacts}/initial-studio.png` });
      await checkStudioControls({ native, button, click, artifacts });
      stories.push('NATIVE-STUDIO-TABS-AND-SPEED');
      await click('button[aria-label="Play"]');
      await native.wait(
        'document.querySelector("video").currentTime > 0.1 && !document.querySelector("video").paused',
        'native video playing',
      );
      await click('button[aria-label="Pause"]');
      await native.wait(
        'document.querySelector("video").paused',
        'native video paused',
      );
      stories.push('NATIVE-STUDIO-PLAY-TIME-ADVANCE-PAUSE');
      if (longPlayback) {
        await checkLongPlayback({ native, click, detail, artifacts });
        stories.push('NATIVE-STUDIO-MULTI-FRAGMENT-FULL-PLAYBACK');
      }
    }
  }
  await viewer.screenshot({ path: `${artifacts}/${phase}.png` });
  process.stdout.write(`${JSON.stringify({ phase, passed: stories })}\n`);
} catch (error) {
  failure = error.message;
  await writeNativeFailureDiagnostics({ native, viewer, artifacts, phase });
  throw error;
} finally {
  await writeFile(
    `${artifacts}/${phase}-report.json`,
    JSON.stringify({ phase, passed: stories, failure }, null, 2),
  );
  if (keyboard) await keyboard.close();
  else native?.close();
  await browser.close();
}
