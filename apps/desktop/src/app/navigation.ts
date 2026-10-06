import {
  Activity,
  Database,
  Home,
  Settings,
  Video,
  ListTodo,
} from 'lucide-react';

export interface NavigationItem {
  description: string;
  href: string;
  icon: typeof Home;
  label: string;
}

export const navigationItems: NavigationItem[] = [
  {
    description:
      'A private workspace for capture, review, and privacy controls.',
    href: '/',
    icon: Home,
    label: 'Home',
  },
  {
    description: 'Choose sources, check permissions, and control recording.',
    href: '/capture',
    icon: Video,
    label: 'Capture',
  },
  {
    description:
      'Review, rename, edit, and remove recordings saved on this device.',
    href: '/library',
    icon: Database,
    label: 'Library',
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
