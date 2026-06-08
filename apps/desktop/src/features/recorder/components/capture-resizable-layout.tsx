import type { ReactNode } from 'react';
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable';

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
  return (
    <ResizablePanelGroup className="min-h-0 flex-1" orientation="horizontal">
      <ResizablePanel defaultSize="22%" minSize="16%">
        {sourcesPanel}
      </ResizablePanel>
      <ResizableHandle withHandle />
      <ResizablePanel defaultSize="52%" minSize="32%">
        <main className="bg-foreground/10 flex h-full flex-col items-center justify-center overflow-auto p-4">
          <div className="bg-foreground/10 w-full">{canvasPanel}</div>
        </main>
      </ResizablePanel>
      <ResizableHandle withHandle />
      <ResizablePanel defaultSize="26%" minSize="20%">
        {geometryPanel}
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}
