import { Activity, LockKeyhole, ScanText, Video } from 'lucide-react';

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
    <section className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-normal">
          Daydream desktop scaffold
        </h1>
        <p className="text-muted-foreground mt-2 max-w-3xl text-sm leading-6">
          This shell is ready for Linux/Wayland-first capture work. The first
          implemented backend command is diagnostics with a real media writer
          probe; recording, storage, redaction, OCR, transcription, and indexing
          remain intentionally stubbed.
        </p>
      </div>

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
    </section>
  );
}
