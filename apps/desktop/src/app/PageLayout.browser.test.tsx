import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, expect, test } from 'vitest';
import {
  FullHeightLayout,
  NarrowPageLayout,
  PageHeader,
  PageLayout,
} from './PageLayout';

afterEach(() => {
  cleanup();
});

test('PageLayout keeps document route spacing consistent', () => {
  render(
    <PageLayout>
      <PageHeader
        title="Layout title"
        description="Layout description"
        actions={<button type="button">Action</button>}
      />
    </PageLayout>,
  );

  const page = screen
    .getByRole('heading', { name: 'Layout title' })
    .closest('section');

  expect(page?.className).toContain(
    'mx-auto flex w-full max-w-5xl flex-col gap-6 p-6',
  );
  expect(screen.getByText('Layout description').className).toContain(
    'text-muted-foreground mt-2 max-w-3xl text-sm leading-6',
  );
  expect(screen.getByRole('button', { name: 'Action' })).toBeTruthy();
});

test('NarrowPageLayout and FullHeightLayout expose route variants', () => {
  render(
    <>
      <NarrowPageLayout>
        <h1>Narrow title</h1>
      </NarrowPageLayout>
      <FullHeightLayout>
        <h1>Full height title</h1>
      </FullHeightLayout>
    </>,
  );

  expect(
    screen.getByRole('heading', { name: 'Narrow title' }).closest('section')
      ?.className,
  ).toContain('mx-auto flex w-full max-w-3xl flex-col gap-3 p-6');
  expect(
    screen
      .getByRole('heading', { name: 'Full height title' })
      .closest('section')?.className,
  ).toContain('flex min-h-full flex-col');
});
