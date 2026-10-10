import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import { Button } from '@/components/ui/button';
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '@/components/ui/resizable';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from '@/components/ui/sheet';

export interface RecordingStudioLayoutProps {
  player: ReactNode;
  timeline: ReactNode;
  inspector: ReactNode;
  initiallyOpen: boolean;
}
export function RecordingStudioLayout({
  player,
  timeline,
  inspector,
  initiallyOpen,
}: RecordingStudioLayoutProps) {
  const [compact, setCompact] = useState(() => window.innerWidth < 1200);
  const [preference, setPreference] = useState<boolean | null>(null);
  const open = preference ?? initiallyOpen;
  const inspectorRef = useRef<HTMLDivElement>(null),
    triggerRef = useRef<HTMLButtonElement>(null),
    outside = useRef(false);
  const changeOpen = useCallback(
    (next: boolean) => {
      if (next === open) return;
      setPreference(next);
      const restore = !outside.current;
      outside.current = false;
      requestAnimationFrame(() => {
        if (next) {
          inspectorRef.current?.focus();
          outside.current = false;
        } else if (restore) triggerRef.current?.focus();
      });
    },
    [open],
  );
  const interactOutside = useCallback(
    (event: Event) => {
      if (!open) event.preventDefault();
      else outside.current = true;
    },
    [open],
  );
  const autofocus = useCallback(
    (event: Event) => {
      if (!open) event.preventDefault();
    },
    [open],
  );
  useEffect(() => {
    const query = window.matchMedia('(max-width: 1199px)');
    function update() {
      setCompact(query.matches);
    }
    query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  }, []);
  return compact ? (
    <>
      <div className="flex justify-end">
        <Sheet modal={false} open={open} onOpenChange={changeOpen}>
          <SheetTrigger asChild>
            <Button
              ref={triggerRef}
              variant="outline"
              data-studio-preserve-selection
            >
              Inspector
            </Button>
          </SheetTrigger>
          <SheetContent
            hidden={!open}
            ref={inspectorRef}
            onOpenAutoFocus={autofocus}
            onInteractOutside={interactOutside}
            forceMount
            className="w-full max-w-[28rem] gap-0 p-0 data-[state=closed]:hidden sm:max-w-[28rem]"
          >
            <SheetHeader className="p-6">
              <SheetTitle>Recording inspector</SheetTitle>
              <SheetDescription>
                Inspect sources, edit annotations and manage redactions.
              </SheetDescription>
            </SheetHeader>
            <div className="min-h-0 flex-1">{inspector}</div>
          </SheetContent>
        </Sheet>
      </div>
      <div className="rounded-workspace min-h-[24rem] overflow-hidden">
        {player}
      </div>
      <div className="rounded-panel min-h-[20rem] flex-1 overflow-hidden border">
        {timeline}
      </div>
    </>
  ) : (
    <ResizablePanelGroup className="min-h-0 flex-1" orientation="horizontal">
      <ResizablePanel defaultSize="68%" minSize="45%">
        <ResizablePanelGroup orientation="vertical">
          <ResizablePanel defaultSize="60%" minSize={260}>
            <div className="rounded-workspace h-full overflow-hidden">
              {player}
            </div>
          </ResizablePanel>
          <ResizableHandle
            className="w-6 bg-transparent aria-[orientation=horizontal]:h-6"
            withHandle
          />
          <ResizablePanel defaultSize="40%" minSize={240}>
            <div className="rounded-panel h-full overflow-hidden border">
              {timeline}
            </div>
          </ResizablePanel>
        </ResizablePanelGroup>
      </ResizablePanel>
      <ResizableHandle
        className="w-6 bg-transparent aria-[orientation=horizontal]:h-6"
        withHandle
      />
      <ResizablePanel defaultSize="32%" minSize={320}>
        {inspector}
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}
