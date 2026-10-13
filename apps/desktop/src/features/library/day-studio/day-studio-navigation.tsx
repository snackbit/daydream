import { useCallback, useState } from 'react';
import { ChevronDown } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from '@/components/ui/sheet';
import { DayReviewPanel } from '@/features/recall/day/day-review-panel';
import { DayRecordingsPanel } from './day-recordings-panel';
import type { DayTimelineModel } from './day-timeline';

export interface DayStudioNavigationProps {
  model: DayTimelineModel;
  activeId: string | null;
  onSelect: (time: number) => void;
}
export function DayStudioNavigation({
  model,
  activeId,
  onSelect,
}: DayStudioNavigationProps) {
  const [open, setOpen] = useState(false);
  const select = useCallback(
    (time: number) => {
      onSelect(time);
      setOpen(false);
    },
    [onSelect],
  );
  return (
    <div className="flex items-center gap-2" data-studio-preserve-selection>
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button variant="outline">
            Recordings <ChevronDown />
          </Button>
        </PopoverTrigger>
        <PopoverContent align="start" className="w-96 p-0">
          <ScrollArea className="max-h-96">
            <DayRecordingsPanel
              model={model}
              activeId={activeId}
              onSelect={select}
            />
          </ScrollArea>
        </PopoverContent>
      </Popover>
      <Sheet>
        <SheetTrigger asChild>
          <Button variant="outline">Day review</Button>
        </SheetTrigger>
        <SheetContent className="data-[side=right]:w-full data-[side=right]:max-w-[56rem] data-[side=right]:sm:max-w-[56rem]">
          <SheetHeader>
            <SheetTitle>Day review</SheetTitle>
          </SheetHeader>
          <ScrollArea className="min-h-0 flex-1">
            <DayReviewPanel date={model.manifest.dayId} />
          </ScrollArea>
        </SheetContent>
      </Sheet>
    </div>
  );
}
