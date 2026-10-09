import { useState } from 'react';
import type { RecorderSceneSource } from '@daydream/shared';
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import '@/index.css';
import * as client from '../recorder-client';
import { SourceCropEditor } from './source-crop-editor';

vi.mock('../recorder-client');
beforeEach(async () => {
  await page.viewport(1440, 960);
  vi.mocked(client.updateRecorderPreviewDemand).mockResolvedValue(undefined);
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  delete document.documentElement.dataset.theme;
});
const initial: RecorderSceneSource = {
  id: 'screen-synthetic',
  sourceId: 'source-synthetic',
  sourceKind: 'monitor',
  displayName: 'Synthetic monitor',
  sourceWidth: 5120,
  sourceHeight: 1440,
  preserveAspectRatio: true,
  zIndex: 0,
  crop: { left: 0, top: 0, right: 0, bottom: 0 },
  rect: { x: 0, y: 0, width: 2560, height: 720 },
};
function Editor() {
  const [source, setSource] = useState(initial);
  return (
    <div style={{ height: 600, display: 'flex', flexDirection: 'column' }}>
      <SourceCropEditor
        source={source}
        frame={undefined}
        disabled={false}
        onChange={setSource}
      />
      <output aria-label="Reviewed crop">
        {source.crop.left},{source.crop.top};{' '}
        {5120 - source.crop.left - source.crop.right}×
        {1440 - source.crop.top - source.crop.bottom}; output{' '}
        {source.rect.width}×{source.rect.height}
      </output>
    </div>
  );
}
test.each(['light', 'dark'] as const)(
  'draw, keyboard resize, cancel and zoom preserve reviewed source pixels in %s',
  async (theme) => {
    document.documentElement.dataset.theme = theme;
    if (theme === 'dark') await page.viewport(768, 960);
    render(<Editor />);
    const region = screen.getByRole('region', { name: 'Crop editor' });
    await waitFor(() =>
      expect(vi.mocked(client.updateRecorderPreviewDemand)).toHaveBeenCalled(),
    );
    vi.spyOn(region, 'setPointerCapture').mockImplementation(() => undefined);
    const box = region.getBoundingClientRect();
    const scale = Math.min(
      region.clientWidth / 5120,
      region.clientHeight / 1440,
    );
    const point = (x: number, y: number) => ({
      clientX: box.left + (region.clientWidth - 5120 * scale) / 2 + x * scale,
      clientY: box.top + (region.clientHeight - 1440 * scale) / 2 + y * scale,
      pointerId: 1,
      button: 0,
    });
    await userEvent.click(screen.getByRole('button', { name: 'Draw crop' }));
    fireEvent.pointerDown(region, point(1000, 200));
    fireEvent.pointerMove(region, point(3000, 1200));
    fireEvent.pointerUp(region, point(3000, 1200));
    expect(screen.getByLabelText('Reviewed crop')).toHaveTextContent(
      '1000,200; 2000×1000; output 1000×500',
    );
    region.focus();
    await userEvent.keyboard('{ArrowRight}');
    expect(screen.getByLabelText('Reviewed crop')).toHaveTextContent(
      '1001,200; 2000×1000',
    );
    await userEvent.keyboard('{Alt>}{ArrowLeft}{/Alt}');
    expect(screen.getByLabelText('Reviewed crop')).toHaveTextContent(
      '1999×1000',
    );
    const rectangle = region.querySelector('[data-crop-handle="move"]');
    if (!rectangle) throw new Error('Crop rectangle absent');
    fireEvent.pointerDown(rectangle, point(1100, 300));
    fireEvent.pointerMove(region, point(1300, 400));
    await userEvent.keyboard('{Escape}');
    expect(screen.getByLabelText('Reviewed crop')).toHaveTextContent(
      '1001,200; 1999×1000',
    );
    const before = screen.getByLabelText('Reviewed crop').textContent;
    fireEvent.wheel(region, {
      deltaY: -500,
      clientX: box.left + box.width / 2,
      clientY: box.top + box.height / 2,
    });
    expect(screen.getByLabelText('Reviewed crop').textContent).toBe(before);
    await waitFor(() =>
      expect(
        vi.mocked(client.updateRecorderPreviewDemand).mock.calls.at(-1)?.[0]
          .width,
      ).toBeGreaterThan(1),
    );
    await page.screenshot({ path: `./__screenshots__/crop-${theme}.png` });
  },
);
