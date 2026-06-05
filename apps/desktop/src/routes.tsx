import {
  createRootRoute,
  createRoute,
  createRouter,
} from '@tanstack/react-router';
import { AppShell } from '@/app/AppShell';
import { CaptureScreen } from '@/screens/CaptureScreen';
import { DiagnosticsScreen } from '@/screens/DiagnosticsScreen';
import { HomeScreen } from '@/screens/HomeScreen';
import { PlaceholderScreen } from '@/screens/PlaceholderScreen';

const rootRoute = createRootRoute({
  component: AppShell,
});

const homeRoute = createRoute({
  component: HomeScreen,
  getParentRoute: () => rootRoute,
  path: '/',
});

const captureRoute = createRoute({
  component: CaptureScreen,
  getParentRoute: () => rootRoute,
  path: '/capture',
});

const libraryRoute = createRoute({
  component: () => (
    <PlaceholderScreen
      title="Library"
      summary="Recordings, redactions, searchable annotations, and timeline playback will land here."
    />
  ),
  getParentRoute: () => rootRoute,
  path: '/library',
});

const diagnosticsRoute = createRoute({
  component: DiagnosticsScreen,
  getParentRoute: () => rootRoute,
  path: '/diagnostics',
});

const settingsRoute = createRoute({
  component: () => (
    <PlaceholderScreen
      title="Settings"
      summary="Capture quality, storage, model, privacy, and retention settings will land here."
    />
  ),
  getParentRoute: () => rootRoute,
  path: '/settings',
});

const routeTree = rootRoute.addChildren([
  homeRoute,
  captureRoute,
  libraryRoute,
  diagnosticsRoute,
  settingsRoute,
]);

export const router = createRouter({ routeTree });

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
