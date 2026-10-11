import { lazy, useCallback } from 'react';
import {
  createRootRoute,
  createRoute,
  createRouter,
  redirect,
} from '@tanstack/react-router';
import type { RecallSourceKind } from '@daydream/shared';
import { AppShell } from '@/app/AppShell';
import { RouteErrorScreen } from '@/features/navigation/components/route-error-screen';
import { RouteNotFoundScreen } from '@/features/navigation/components/route-not-found-screen';
import {
  validateRecallDate,
  validateRecallSearch,
  validateStudioSearch,
} from '@/features/navigation/recall-location';
import {
  validateSettingsLocation,
  type SettingsSection,
  type SettingsDetail,
} from '@/features/settings/settings-location';
import { reportFrontendFailure } from '@/lib/diagnostics/failure-logging';

const DayScreen = lazy(() =>
  import('@/screens/DayScreen').then(({ DayScreen }) => ({
    default: DayScreen,
  })),
);
const SearchScreen = lazy(() =>
  import('@/screens/SearchScreen').then(({ SearchScreen }) => ({
    default: SearchScreen,
  })),
);
const CaptureScreen = lazy(() =>
  import('@/screens/CaptureScreen').then(({ CaptureScreen }) => ({
    default: CaptureScreen,
  })),
);
const DiagnosticsScreen = lazy(() =>
  import('@/screens/DiagnosticsScreen').then(({ DiagnosticsScreen }) => ({
    default: DiagnosticsScreen,
  })),
);
const LibraryScreen = lazy(() =>
  import('@/screens/LibraryScreen').then(({ LibraryScreen }) => ({
    default: LibraryScreen,
  })),
);
const NotificationsScreen = lazy(() =>
  import('@/screens/NotificationsScreen').then(({ NotificationsScreen }) => ({
    default: NotificationsScreen,
  })),
);
const ProcessingScreen = lazy(() =>
  import('@/screens/ProcessingScreen').then(({ ProcessingScreen }) => ({
    default: ProcessingScreen,
  })),
);
const RecordingStudioScreen = lazy(() =>
  import('@/screens/RecordingStudioScreen').then(
    ({ RecordingStudioScreen }) => ({ default: RecordingStudioScreen }),
  ),
);
const SettingsScreen = lazy(() =>
  import('@/screens/SettingsScreen').then(({ SettingsScreen }) => ({
    default: SettingsScreen,
  })),
);

const rootRoute = createRootRoute({
  component: AppShell,
  errorComponent: RouteErrorScreen,
  notFoundComponent: RouteNotFoundScreen,
  onCatch: (error) => reportFrontendFailure('route_error', 'render', error),
  onError: (error: unknown) =>
    reportFrontendFailure('route_error', 'render', error),
});

const homeRoute = createRoute({
  beforeLoad: () => {
    throw redirect({ to: '/library' });
  },
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
  validateSearch: validateStudioSearch,
});

const dayRoute = createRoute({
  component: DayRouteScreen,
  getParentRoute: () => rootRoute,
  path: '/day/$date',
  params: { parse: (params) => ({ date: validateRecallDate(params.date) }) },
});

function DayRouteScreen() {
  const { date } = dayRoute.useParams();
  return <DayScreen date={date} />;
}

const searchRoute = createRoute({
  component: SearchRouteScreen,
  getParentRoute: () => rootRoute,
  path: '/search',
  validateSearch: validateRecallSearch,
});

function SearchRouteScreen() {
  const search = searchRoute.useSearch();
  const navigate = searchRoute.useNavigate();
  const changeSearch = useCallback(
    (next: {
      q: string;
      source: RecallSourceKind | 'all';
      date: string | null;
    }) => {
      void navigate({
        search: { ...next, source: next.source === 'all' ? null : next.source },
      });
    },
    [navigate],
  );
  return (
    <SearchScreen
      query={search.q}
      source={search.source ?? 'all'}
      date={search.date}
      onSearchChange={changeSearch}
    />
  );
}

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
  const changeLocation = useCallback(
    (section: SettingsSection, detail: SettingsDetail | null) => {
      void navigate({
        search: {
          section,
          detail,
          tab:
            section === 'recording' || section === 'privacy'
              ? 'record'
              : section === 'models'
                ? 'ai'
                : section,
          focus: null,
        },
      });
    },
    [navigate],
  );
  return (
    <SettingsScreen
      section={search.section}
      detail={search.detail}
      onLocationChange={changeLocation}
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
  dayRoute,
  searchRoute,
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
