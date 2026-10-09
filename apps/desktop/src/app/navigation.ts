import {
  Activity,
  Home,
  Settings,
  Video,
  ListTodo,
  Search,
} from 'lucide-react';

export interface NavigationItem {
  description: string;
  href: string;
  icon: typeof Home;
  label: string;
}

export const navigationItems: NavigationItem[] = [
  {
    description: 'Review Days and recordings saved on this device.',
    href: '/library',
    icon: Home,
    label: 'Library',
  },
  {
    description: 'Choose sources, check permissions, and control recording.',
    href: '/capture',
    icon: Video,
    label: 'Capture',
  },
  {
    description: 'Find text and meaning in eligible screen and audio evidence.',
    href: '/search',
    icon: Search,
    label: 'Search',
  },
  {
    description: 'Progress, background work, and processing recovery.',
    href: '/processing',
    icon: ListTodo,
    label: 'Processing',
  },
  {
    description: 'Runtime health, native capture support, and storage details.',
    href: '/diagnostics',
    icon: Activity,
    label: 'Diagnostics',
  },
  {
    description:
      'Capture quality, storage, local models, and optional remote providers.',
    href: '/settings',
    icon: Settings,
    label: 'Settings',
  },
];
