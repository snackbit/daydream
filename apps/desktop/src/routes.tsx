import {
  createRootRoute,
  createRoute,
  createRouter,
} from '@tanstack/react-router';
import { AppShell } from '@/app/AppShell';
import { CaptureScreen } from '@/screens/CaptureScreen';
import { DiagnosticsScreen } from '@/screens/DiagnosticsScreen';
import { HomeScreen } from '@/screens/HomeScreen';
import { LibraryScreen } from '@/screens/LibraryScreen';
import { RecordingStudioScreen } from '@/screens/RecordingStudioScreen';
import { SettingsScreen } from '@/screens/SettingsScreen';
import { unknownErrorMessage } from './lib/async-store-action';

const rootRoute = createRootRoute({
  component: AppShell,
  errorComponent: ({ error }) => <RouteErrorScreen error={error} />,
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

const routeTree = rootRoute.addChildren([
  homeRoute,
  captureRoute,
  libraryRoute,
  recordingStudioRoute,
  diagnosticsRoute,
  settingsRoute,
]);

export const router = createRouter({ routeTree });

function RouteErrorScreen({ error }: { error: unknown }) {
  return (
    <div className="mx-auto flex max-w-3xl flex-col gap-3 p-6">
      <h1 className="text-xl font-semibold tracking-normal">
        Something failed
      </h1>
      <p className="text-muted-foreground text-sm">
        The last action failed before the screen could recover.
      </p>
      <pre className="bg-muted overflow-auto border p-3 text-xs">
        {unknownErrorMessage(error)}
      </pre>
    </div>
  );
}

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
