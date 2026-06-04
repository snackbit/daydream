import { Activity, Database, Home, Settings, Video } from 'lucide-react';

export interface NavigationItem {
  description: string;
  href: string;
  icon: typeof Home;
  label: string;
}

export const navigationItems: NavigationItem[] = [
  {
    description: 'Default workspace and capture readiness overview.',
    href: '/',
    icon: Home,
    label: 'Home',
  },
  {
    description: 'Future record, pause, stop, and capture profile controls.',
    href: '/capture',
    icon: Video,
    label: 'Capture',
  },
  {
    description:
      'Future recordings, redactions, search, and playback timeline.',
    href: '/library',
    icon: Database,
    label: 'Library',
  },
  {
    description: 'Runtime versions, binary paths, permissions, and probes.',
    href: '/diagnostics',
    icon: Activity,
    label: 'Diagnostics',
  },
  {
    description:
      'Future capture quality, storage, models, and privacy settings.',
    href: '/settings',
    icon: Settings,
    label: 'Settings',
  },
];
