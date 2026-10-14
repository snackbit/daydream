import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { cn } from '@/lib/utils';
import {
  STUDIO_TIMELINE_COLORS,
  studioColorById,
  type StudioLaneDraft,
} from '../../timeline/state/studio-timeline-store';

export function EditableLaneFields({
  lane,
  onColorChange,
  onRename,
}: {
  lane: StudioLaneDraft;
  onColorChange: (colorId: string) => void;
  onRename: (label: string) => void;
}) {
  const color = studioColorById(lane.colorId);

  return (
    <div className="grid gap-2">
      <label className="grid gap-1">
        <span className="text-muted-foreground">Name</span>
        <Input
          aria-label="Track name"
          value={lane.label}
          onChange={(event) => onRename(event.currentTarget.value)}
        />
      </label>
      <div className="grid gap-1">
        <div className="text-muted-foreground">Color</div>
        <ColorPalettePopover
          selectedColorId={color.id}
          onColorChange={onColorChange}
        />
      </div>
    </div>
  );
}

function ColorPalettePopover({
  onColorChange,
  selectedColorId,
}: {
  onColorChange: (colorId: string) => void;
  selectedColorId: string;
}) {
  const selectedColor = studioColorById(selectedColorId);

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button
          aria-label="Select track color"
          className="w-full justify-start"
          type="button"
          variant="outline"
        >
          <span
            aria-hidden="true"
            className="size-3 rounded-full"
            style={{ backgroundColor: selectedColor.strong }}
          />
          {selectedColor.label}
        </Button>
      </PopoverTrigger>
      <PopoverContent align="start" className="w-56">
        <div className="grid grid-cols-6 gap-1">
          {STUDIO_TIMELINE_COLORS.map((color) => (
            <button
              aria-label={color.label}
              aria-pressed={color.id === selectedColorId}
              className={cn(
                'focus-visible:ring-ring size-7 rounded-md border transition-transform outline-none focus-visible:ring-2',
                color.id === selectedColorId && 'ring-ring ring-2',
              )}
              key={color.id}
              style={{
                backgroundColor: color.strong,
                borderColor: color.border,
              }}
              type="button"
              onClick={() => onColorChange(color.id)}
            />
          ))}
        </div>
      </PopoverContent>
    </Popover>
  );
}

export function trackKindLabel(kind: StudioLaneDraft['kind']): string {
  switch (kind) {
    case 'activity-focus':
      return 'Activity events';
    case 'activity-keyboard':
      return 'Keyboard activity';
    case 'activity-mouse':
      return 'Mouse activity';
    case 'desktop-audio':
      return 'Desktop audio';
    case 'microphone':
      return 'Microphone';
    case 'video':
      return 'Video';
  }
}
