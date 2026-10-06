import { useCallback } from 'react';
import {
  createRootRoute,
  createRoute,
  createRouter,
  lazyRouteComponent,
} from '@tanstack/react-router';
import { AppShell } from '@/app/AppShell';
import { RouteErrorScreen } from '@/features/navigation/components/route-error-screen';
import { RouteNotFoundScreen } from '@/features/navigation/components/route-not-found-screen';
import {
  validateSettingsLocation,
  type SettingsTab,
} from '@/features/settings/settings-location';
import { HomeScreen } from '@/screens/HomeScreen';

const CaptureScreen = lazyRouteComponent(
  () => import('@/screens/CaptureScreen'),
  'CaptureScreen',
);
const DiagnosticsScreen = lazyRouteComponent(
  () => import('@/screens/DiagnosticsScreen'),
  'DiagnosticsScreen',
);
const LibraryScreen = lazyRouteComponent(
  () => import('@/screens/LibraryScreen'),
  'LibraryScreen',
);
const NotificationsScreen = lazyRouteComponent(
  () => import('@/screens/NotificationsScreen'),
  'NotificationsScreen',
);
const ProcessingScreen = lazyRouteComponent(
  () => import('@/screens/ProcessingScreen'),
  'ProcessingScreen',
);
const RecordingStudioScreen = lazyRouteComponent(
  () => import('@/screens/RecordingStudioScreen'),
  'RecordingStudioScreen',
);
const SettingsScreen = lazyRouteComponent(
  () => import('@/screens/SettingsScreen'),
  'SettingsScreen',
);

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
  component: SettingsRouteScreen,
  getParentRoute: () => rootRoute,
  path: '/settings',
  validateSearch: validateSettingsLocation,
});

function SettingsRouteScreen() {
  const search = settingsRoute.useSearch();
  const navigate = settingsRoute.useNavigate();
  const changeTab = useCallback(
    (tab: SettingsTab) => {
      void navigate({ search: { tab, focus: null } });
    },
    [navigate],
  );
  return (
    <SettingsScreen
      tab={search.tab}
      focusSection={search.focus}
      onTabChange={changeTab}
    />
  );
}

const notificationsRoute = createRoute({
  component: NotificationsScreen,
  getParentRoute: () => rootRoute,
  path: '/notifications',
});

const processingRoute = createRoute({
  component: ProcessingScreen,
  getParentRoute: () => rootRoute,
  path: '/processing',
});

const routeTree = rootRoute.addChildren([
  homeRoute,
  captureRoute,
  libraryRoute,
  recordingStudioRoute,
  diagnosticsRoute,
  settingsRoute,
  notificationsRoute,
  processingRoute,
]);

export const router = createRouter({ routeTree });

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
