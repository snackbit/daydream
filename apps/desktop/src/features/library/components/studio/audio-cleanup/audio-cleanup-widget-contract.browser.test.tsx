import { useLayoutEffect, useRef, type ComponentProps } from 'react';
import { cleanup, render, screen } from '@testing-library/react';
import type * as Radix from 'radix-ui';
import { afterEach, expect, test, vi } from 'vitest';
import { userEvent } from 'vitest/browser';
import { AudioCleanupDialog } from './audio-cleanup-dialog';
import { AudioCleanupComparison } from './audio-cleanup-preview';
import type { AudioCleanupController } from './use-audio-cleanup';

// Keep the shared controls real. Only the external widget callback is fault-injected.
vi.mock('radix-ui', async (importOriginal) => {
  const original = await importOriginal<typeof Radix>();
  function SliderRoot(props: ComponentProps<typeof original.Slider.Root>) {
    function malformed() {
      props.onValueChange?.([]);
    }
    return (
      <>
        <original.Slider.Root {...props} />
        <button
          onClick={malformed}
          aria-label={`Empty ${props['aria-label'] ?? 'slider'}`}
        >
          Empty widget update
        </button>
      </>
    );
  }
  function SelectRoot(props: ComponentProps<typeof original.Select.Root>) {
    function malformed() {
      props.onValueChange?.('unsupported-mode');
    }
    return (
      <>
        <original.Select.Root {...props} />
        <button onClick={malformed}>Unsupported mode update</button>
      </>
    );
  }
  function TabsRoot(props: ComponentProps<typeof original.Tabs.Root>) {
    const onValueChange = props.onValueChange;
    const initialized = useRef(false);
    useLayoutEffect(() => {
      if (!initialized.current) {
        initialized.current = true;
        onValueChange?.('original');
      }
    }, [onValueChange]);
    function malformed() {
      props.onValueChange?.('unsupported-preview');
    }
    return (
      <>
        <original.Tabs.Root {...props} />
        <button onClick={malformed}>Unsupported preview update</button>
      </>
    );
  }
  return {
    ...original,
    Slider: { ...original.Slider, Root: SliderRoot },
    Select: { ...original.Select, Root: SelectRoot },
    Tabs: { ...original.Tabs, Root: TabsRoot },
  };
});
afterEach(cleanup);
test('unexpected empty numeric values and unsupported mode never mutate cleanup settings', async () => {
  const change = vi.fn();
  const config = { mode: 'auto' as const, matchThreshold: 0.65, amount: 1 };
  const controller: AudioCleanupController = {
    opened: true,
    state: {
      recordingId: 'synthetic',
      config,
      pendingConfig: null,
      revision: null,
      phase: 'idle',
      progress: 0,
      matchedMs: 0,
      referenceAvailable: true,
      transcriptsOutdated: false,
      error: null,
    },
    draft: config,
    preview: null,
    busy: null,
    error: null,
    open: vi.fn(),
    close: vi.fn(),
    change,
    compare: vi.fn(),
    apply: vi.fn(),
    cancel: vi.fn(),
  };
  render(
    <AudioCleanupDialog
      controller={controller}
      onRegenerate={vi.fn()}
      unavailableReason={null}
    />,
  );
  await userEvent.click(
    screen.getByRole('button', { name: 'Unsupported mode update' }),
  );
  await userEvent.click(screen.getByRole('button', { name: 'Empty slider' }));
  await userEvent.click(
    screen.getByRole('button', { name: 'Advanced tuning' }),
  );
  for (const button of screen.getAllByRole('button', { name: 'Empty slider' }))
    await userEvent.click(button);
  expect(change).not.toHaveBeenCalled();
});
test('preview initialization before media attachment and unsupported tabs keep a valid source', async () => {
  render(
    <AudioCleanupComparison
      preview={{
        recordingId: 'synthetic',
        sessionId: 'preview',
        originalUrl: 'http://127.0.0.1:54321/playback/source/original',
        cleanedUrl: 'http://127.0.0.1:54321/playback/source/cleaned',
        matchedMs: 0,
        durationMs: 5000,
      }}
    />,
  );
  await userEvent.click(
    screen.getByRole('button', { name: 'Unsupported preview update' }),
  );
  expect(screen.getByRole('tab', { name: 'Original' })).toHaveAttribute(
    'aria-selected',
    'true',
  );
  expect(document.querySelector('audio')?.src).toContain('/original');
});
