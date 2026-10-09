import { PageHeader, PageLayout } from '@/app/PageLayout';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { LibraryRecordingsContent } from '@/features/library/components/overview/library-recordings-content';
import { LibraryDays } from '@/features/recall/library/library-days';

export function LibraryScreen() {
  return (
    <PageLayout>
      <PageHeader
        title="Library"
        description="A place to return to the work you left behind."
      />
      <Tabs defaultValue="days">
        <TabsList>
          <TabsTrigger value="days">Days</TabsTrigger>
          <TabsTrigger value="recordings">Recordings</TabsTrigger>
        </TabsList>
        <TabsContent value="days">
          <LibraryDays />
        </TabsContent>
        <TabsContent value="recordings">
          <LibraryRecordingsContent />
        </TabsContent>
      </Tabs>
    </PageLayout>
  );
}
