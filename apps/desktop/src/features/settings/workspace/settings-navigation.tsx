import {
  SlidersHorizontal,
  Video,
  Shield,
  Sparkles,
  Boxes,
  HardDrive,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { settingsSection, type SettingsSection } from '../settings-location';

export const settingsSections = [
  { id: 'general', label: 'General', icon: SlidersHorizontal },
  { id: 'recording', label: 'Recording', icon: Video },
  { id: 'privacy', label: 'Privacy', icon: Shield },
  { id: 'ai', label: 'AI features', icon: Sparkles },
  { id: 'models', label: 'Models & accounts', icon: Boxes },
  { id: 'storage', label: 'Storage', icon: HardDrive },
] satisfies { id: SettingsSection; label: string; icon: typeof Video }[];

export interface SettingsNavigationProps {
  section: SettingsSection;
  onChange: (section: SettingsSection) => void;
}
function SectionLink({
  item,
  active,
  onChange,
}: {
  item: (typeof settingsSections)[number];
  active: boolean;
  onChange: SettingsNavigationProps['onChange'];
}) {
  function select() {
    onChange(item.id);
  }
  const Icon = item.icon;
  return (
    <Button
      type="button"
      variant="ghost"
      aria-current={active ? 'page' : undefined}
      onClick={select}
      className="aria-[current=page]:bg-muted w-full justify-start gap-2.5 rounded-lg px-3 text-[13px] font-normal aria-[current=page]:font-medium"
    >
      <Icon aria-hidden="true" className="size-4 shrink-0" />
      {item.label}
    </Button>
  );
}
export function SettingsNavigation({
  section,
  onChange,
}: SettingsNavigationProps) {
  function select(value: string) {
    const next = settingsSection(value);
    if (next) onChange(next);
  }
  return (
    <>
      <nav
        aria-label="Settings sections"
        className="sticky top-0 hidden self-start lg:grid lg:gap-1"
      >
        {settingsSections.map((item) => (
          <SectionLink
            key={item.id}
            item={item}
            active={section === item.id}
            onChange={onChange}
          />
        ))}
      </nav>
      <div className="lg:hidden">
        <Select value={section} onValueChange={select}>
          <SelectTrigger aria-label="Settings section" className="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {settingsSections.map((item) => (
              <SelectItem key={item.id} value={item.id}>
                {item.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
    </>
  );
}
