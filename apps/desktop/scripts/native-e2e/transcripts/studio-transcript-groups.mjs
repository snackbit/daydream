import { readFile } from 'node:fs/promises';
import { join } from 'node:path';

// Real WebKit UI -> typed preview IPC -> eligible SQLite excerpts. All source
// content comes from the runner's synthetic eight-hour fixture.
export async function checkStudioTranscriptGroups({
  native,
  click,
  viewer,
  artifacts,
}) {
  async function markGroup(attribute, focus = false) {
    await native.wait(
      `(() => {
      document.querySelectorAll('[${attribute}]').forEach(element=>element.removeAttribute('${attribute}'));
      const viewport=document.querySelector('[aria-label="Track timeline"]').closest('[data-slot="scroll-area-viewport"]'),bounds=viewport.getBoundingClientRect();
      const labelRight=viewport.querySelector('[data-track-id]:not([data-lane-content])')?.getBoundingClientRect().right??bounds.left;
      const group=[...document.querySelectorAll('button')].find(button=>{
        if(!/^Zoom into \\d+ transcript entries$/.test(button.getAttribute('aria-label')??''))return false;
        const rect=button.getBoundingClientRect();
        return rect.width>0&&rect.height>0&&rect.x>=labelRight&&rect.right<=bounds.right;
      });
      if(!group)return false;
      const rect=group.getBoundingClientRect();
      viewport.scrollTop+=rect.y+rect.height/2-bounds.y-viewport.clientHeight/2;
      const visible=group.getBoundingClientRect();
      if(!group.contains(document.elementFromPoint(visible.x+visible.width/2,visible.y+visible.height/2)))return false;
      group.setAttribute('${attribute}','true');
      if(${focus}){group.blur();group.focus({preventScroll:true});}
      return true;
    })()`,
      'visible native transcript count available',
    );
  }
  if (
    await native.evaluate(
      'Boolean(document.querySelector(\'button[aria-label="Pause"]\'))',
    )
  )
    await click('button[aria-label="Pause"]');
  await native.evaluate(
    `document.querySelector('[aria-label="Track timeline"]').closest('[data-slot="scroll-area-viewport"]').scrollIntoView({block:'end',behavior:'instant'}); true`,
  );
  for (let cycle = 0; cycle < 2; cycle++) {
    await markGroup('data-native-transcript-group', true);
    await native.wait(
      `document.querySelector('[role=tooltip]')?.textContent.includes('Synthetic spoken entry')`,
      'native tooltip loads eligible transcript excerpts',
    );
    if (cycle === 0)
      await viewer.screenshot({
        path: join(artifacts, 'studio-transcript-tooltip.png'),
      });
    // Publishing the preview can replace its keyed strip while the tooltip
    // loads. Locate the current count after that publication before clicking.
    await markGroup('data-native-transcript-group');
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
      await markGroup('data-native-transcript-detail-group');
      await click('[data-native-transcript-detail-group]');
    }
    const individual = await native.evaluate(
      'Boolean(document.querySelector("button[data-transcript-segment-id]"))',
    );
    if (individual) {
      await click('button[data-transcript-segment-id]');
      await native.wait(
        `[...document.querySelectorAll('[role=region]')].some(region=>region.getAttribute('aria-label')?.includes('transcript')&&region.textContent.includes('Synthetic spoken entry'))`,
        'native transcript opens the inspector',
      );
    } else {
      // A 400 ms utterance can remain grouped at the existing ten-second zoom
      // limit in a narrow pane. Its eligible text must still be readable.
      await native.wait(
        'document.querySelector(\'button[aria-label="Zoom in tracks"]\').disabled',
        'short utterances reach the existing zoom limit',
      );
      await markGroup('data-native-transcript-detail-group', true);
      await native.wait(
        `document.querySelector('[role=tooltip]')?.textContent.includes('Synthetic spoken entry')`,
        'grouped short utterances retain eligible excerpts',
      );
    }
    if (cycle === 0)
      await viewer.screenshot({
        path: join(artifacts, 'studio-transcript-detail.png'),
      });
    for (let i = 0; i < 8; i++)
      await click('button[aria-label="Zoom out tracks"]');
  }
}

export async function checkSeekedStudioTranscripts({
  native,
  navigate,
  click,
  viewer,
  artifacts,
}) {
  const fixture = JSON.parse(
    await readFile(join(artifacts, 'studio-fixture.json'), 'utf8'),
  );
  await navigate('Library', '/library');
  const selector = `a[href="/day/${fixture.date}"]`;
  await native.wait(
    `document.querySelector(${JSON.stringify(selector)})!==null`,
    'fixture day available',
  );
  await click(selector);
  await native.wait(
    'document.querySelector("video")?.readyState>=2',
    'fixture frame decoded',
  );
  await click('[aria-label="Playback position"]', { fractionX: 0.22 });
  await native.wait(
    '!document.body.textContent.includes("Loading details")',
    'seeked result window loaded',
  );
  await checkStudioTranscriptGroups({ native, click, viewer, artifacts });
}
