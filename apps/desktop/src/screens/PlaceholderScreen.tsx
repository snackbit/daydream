import { PageHeader, PageLayout } from '@/app/PageLayout';

export interface PlaceholderScreenProps {
  summary: string;
  title: string;
}

export function PlaceholderScreen({ summary, title }: PlaceholderScreenProps) {
  return (
    <PageLayout>
      <PageHeader title={title} description={summary} />
      <div className="bg-card text-muted-foreground border p-4 text-sm">
        Placeholder only. The scaffold keeps the navigation surface in place
        without pretending this feature is implemented.
      </div>
    </PageLayout>
  );
}
