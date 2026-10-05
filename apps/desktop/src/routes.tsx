import {
  createRootRoute,
  createRoute,
  createRouter,
} from '@tanstack/react-router';
import { AppShell } from '@/app/AppShell';
import { RouteErrorScreen } from '@/features/navigation/components/route-error-screen';
import { RouteNotFoundScreen } from '@/features/navigation/components/route-not-found-screen';
import { CaptureScreen } from '@/screens/CaptureScreen';
import { DiagnosticsScreen } from '@/screens/DiagnosticsScreen';
import { HomeScreen } from '@/screens/HomeScreen';
import { LibraryScreen } from '@/screens/LibraryScreen';
import { NotificationsScreen } from '@/screens/NotificationsScreen';
import { RecordingStudioScreen } from '@/screens/RecordingStudioScreen';
import { SettingsScreen } from '@/screens/SettingsScreen';

const rootRoute = createRootRoute({
  component: AppShell,
  errorComponent: RouteErrorScreen,
  notFoundComponent: RouteNotFoundScreen,
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
  component: LibraryScreen,
  getParentRoute: () => rootRoute,
  path: '/library',
});

const recordingStudioRoute = createRoute({
  component: RecordingStudioScreen,
  getParentRoute: () => rootRoute,
  path: '/library/$recordingId',
});

const diagnosticsRoute = createRoute({
  component: DiagnosticsScreen,
  getParentRoute: () => rootRoute,
  path: '/diagnostics',
});

const settingsRoute = createRoute({
  component: SettingsScreen,
  getParentRoute: () => rootRoute,
  path: '/settings',
});

const notificationsRoute = createRoute({
  component: NotificationsScreen,
  getParentRoute: () => rootRoute,
  path: '/notifications',
});

const routeTree = rootRoute.addChildren([
  homeRoute,
  captureRoute,
  libraryRoute,
  recordingStudioRoute,
  diagnosticsRoute,
  settingsRoute,
  notificationsRoute,
]);

export const router = createRouter({ routeTree });

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
