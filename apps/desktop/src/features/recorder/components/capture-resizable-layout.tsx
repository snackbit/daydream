import type { ReactNode } from 'react';
import { useLayoutEffect, useRef, useState } from 'react';
import { AudioLines, Monitor, SlidersHorizontal } from 'lucide-react';
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';

export interface CaptureResizableLayoutProps {
  canvasPanel: ReactNode;
  geometryPanel: ReactNode;
  sourcesPanel: ReactNode;
}

export function CaptureResizableLayout({
  canvasPanel,
  geometryPanel,
  sourcesPanel,
}: CaptureResizableLayoutProps) {
  const layoutRef = useRef<HTMLDivElement>(null);
  const [compact, setCompact] = useState(false);

  useLayoutEffect(() => {
    const element = layoutRef.current;
    if (!element) {
      return;
    }

    const updateLayout = (width: number) => setCompact(width < 896);
    updateLayout(element.getBoundingClientRect().width);

    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry) {
        updateLayout(entry.contentRect.width);
      }
    });
    observer.observe(element);

    return () => observer.disconnect();
  }, []);

  return (
    <div className="flex h-full min-h-0 w-full flex-1" ref={layoutRef}>
      {compact ? (
        <Tabs
          className="flex min-h-0 flex-1 flex-col gap-0"
          defaultValue="sources"
        >
          <TabsList
            aria-label="Capture workspace panels"
            className="grid h-auto w-full shrink-0 grid-cols-3 rounded-none border-b p-1"
          >
            <TabsTrigger className="h-9 gap-1.5 px-2 text-xs" value="sources">
              <AudioLines aria-hidden="true" className="size-3.5" />
              Sources
            </TabsTrigger>
            <TabsTrigger className="h-9 gap-1.5 px-2 text-xs" value="preview">
              <Monitor aria-hidden="true" className="size-3.5" />
              Preview
            </TabsTrigger>
            <TabsTrigger className="h-9 gap-1.5 px-2 text-xs" value="geometry">
              <SlidersHorizontal aria-hidden="true" className="size-3.5" />
              Layout
            </TabsTrigger>
          </TabsList>
          <TabsContent
            className="min-h-0 flex-1 overflow-hidden"
            value="sources"
          >
            {sourcesPanel}
          </TabsContent>
          <TabsContent
            className="min-h-0 flex-1 overflow-hidden"
            value="preview"
          >
            <section
              aria-label="Scene preview"
              className="bg-muted/40 flex h-full min-h-[18rem] flex-col items-center justify-center overflow-auto p-2"
            >
              <div className="bg-card w-full overflow-hidden border">
                {canvasPanel}
              </div>
            </section>
          </TabsContent>
          <TabsContent
            className="min-h-0 flex-1 overflow-hidden"
            value="geometry"
          >
            {geometryPanel}
          </TabsContent>
        </Tabs>
      ) : (
        <ResizablePanelGroup
          className="min-h-0 flex-1"
          orientation="horizontal"
        >
          <ResizablePanel defaultSize="22%" minSize="16%">
            {sourcesPanel}
          </ResizablePanel>
          <ResizableHandle withHandle />
          <ResizablePanel defaultSize="52%" minSize="32%">
            <section
              aria-label="Scene preview"
              className="bg-muted/40 flex h-full flex-col items-center justify-center overflow-auto p-2"
            >
              <div className="bg-card w-full overflow-hidden border">
                {canvasPanel}
              </div>
            </section>
          </ResizablePanel>
          <ResizableHandle withHandle />
          <ResizablePanel defaultSize="26%" minSize="20%">
            {geometryPanel}
          </ResizablePanel>
        </ResizablePanelGroup>
      )}
    </div>
  );
}
