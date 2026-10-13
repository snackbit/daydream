import { join } from 'node:path';

// Real WebKit UI -> typed preview IPC -> eligible SQLite excerpts. All source
// content comes from the runner's synthetic eight-hour fixture.
export async function checkStudioTranscriptGroups({
  native,
  click,
  viewer,
  artifacts,
}) {
  if (
    await native.evaluate(
      'Boolean(document.querySelector(\'button[aria-label="Pause"]\'))',
    )
  )
    await click('button[aria-label="Pause"]');
  await native.evaluate(
    `document.querySelector('[aria-label="Track timeline"]').scrollIntoView({block:'end',behavior:'instant'}); true`,
  );
  for (let cycle = 0; cycle < 2; cycle++) {
    await native.wait(
      `(() => { const group=[...document.querySelectorAll('button')].find(button=>/^Zoom into \\d+ transcript entries$/.test(button.getAttribute('aria-label')??'')); if(!group)return false; group.setAttribute('data-native-transcript-group','true'); group.focus(); return true; })()`,
      'native transcript count available',
    );
    await native.wait(
      `document.querySelector('[role=tooltip]')?.textContent.includes('Synthetic spoken entry')`,
      'native tooltip loads eligible transcript excerpts',
    );
    if (cycle === 0)
      await viewer.screenshot({
        path: join(artifacts, 'studio-transcript-tooltip.png'),
      });
    await click('[data-native-transcript-group]');
    for (let attempt = 0; attempt < 3; attempt++) {
      await native.wait(
        `!document.body.textContent.includes('Loading details…')`,
        'native transcript detail handoff settles',
      );
      if (
        await native.evaluate(
          'Boolean(document.querySelector("button[data-transcript-segment-id]"))',
        )
      )
        break;
      await native.wait(
        `(() => { const group=[...document.querySelectorAll('button')].find(button=>/^Zoom into \\d+ transcript entries$/.test(button.getAttribute('aria-label')??'')); if(!group)return false; group.setAttribute('data-native-transcript-detail-group','true'); return true; })()`,
        'short utterances retain a zoomable group',
      );
      await click('[data-native-transcript-detail-group]');
    }
    await native.wait(
      `document.querySelector('button[data-transcript-segment-id]')?.textContent.includes('Synthetic spoken entry')`,
      'zoom reveals real transcript text',
    );
    await native.evaluate(
      `document.querySelector('button[data-transcript-segment-id]').scrollIntoView({block:'nearest',inline:'nearest'}); true`,
    );
    await click('button[data-transcript-segment-id]');
    await native.wait(
      `[...document.querySelectorAll('[role=region]')].some(region=>region.getAttribute('aria-label')?.includes('transcript')&&region.textContent.includes('Synthetic spoken entry'))`,
      'native transcript opens the inspector',
    );
    if (cycle === 0)
      await viewer.screenshot({
        path: join(artifacts, 'studio-transcript-detail.png'),
      });
    for (let i = 0; i < 8; i++)
      await click('button[aria-label="Zoom out tracks"]');
  }
}
