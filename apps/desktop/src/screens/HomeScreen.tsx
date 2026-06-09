import { Activity, LockKeyhole, ScanText, Video } from 'lucide-react';
import { PageHeader, PageLayout } from '@/app/PageLayout';

const readinessItems = [
  {
    icon: Video,
    label: 'Capture',
    value: 'Planned',
  },
  {
    icon: ScanText,
    label: 'OCR every retained frame',
    value: 'Planned',
  },
  {
    icon: Activity,
    label: 'Activity and window metadata',
    value: 'Planned',
  },
  {
    icon: LockKeyhole,
    label: 'Local-first privacy',
    value: 'Default',
  },
];

export function HomeScreen() {
  return (
    <PageLayout>
      <PageHeader
        title="Daydream desktop scaffold"
        description={
          <>
            This shell is ready for Linux/Wayland-first capture work. The first
            implemented backend command is diagnostics with a real media writer
            probe; recording, storage, redaction, OCR, transcription, and
            indexing remain intentionally stubbed.
          </>
        }
      />

      <div className="grid gap-3 md:grid-cols-2">
        {readinessItems.map((item) => {
          const Icon = item.icon;

          return (
            <div
              className="bg-card text-card-foreground border p-4"
              key={item.label}
            >
              <div className="flex items-center justify-between gap-3">
                <div className="flex items-center gap-2">
                  <Icon className="text-primary size-4" />
                  <span className="text-sm font-medium">{item.label}</span>
                </div>
                <span className="text-muted-foreground text-xs">
                  {item.value}
                </span>
              </div>
            </div>
          );
        })}
      </div>
    </PageLayout>
  );
}
