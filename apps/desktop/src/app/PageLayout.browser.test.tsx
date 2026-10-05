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

test('PageLayout presents a semantic page heading, description, and action', () => {
  render(
    <PageLayout>
      <PageHeader
        title="Layout title"
        description="Layout description"
        actions={<button type="button">Action</button>}
      />
    </PageLayout>,
  );

  expect(
    screen.getByRole('heading', { name: 'Layout title', level: 1 }),
  ).toBeTruthy();
  expect(screen.getByText('Layout description')).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Action' })).toBeTruthy();
});

test('NarrowPageLayout and FullHeightLayout retain their page content', () => {
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

  expect(screen.getByRole('heading', { name: 'Narrow title' })).toBeTruthy();
  expect(
    screen.getByRole('heading', { name: 'Full height title' }),
  ).toBeTruthy();
});
