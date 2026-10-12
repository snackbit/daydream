import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { playbackSpeeds, type PlaybackSpeed } from './playback-speed';

export interface PlaybackSpeedControlProps {
  value: PlaybackSpeed;
  onChange: (value: PlaybackSpeed) => void;
}
export function PlaybackSpeedControl({
  value,
  onChange,
}: PlaybackSpeedControlProps) {
  function change(value: string) {
    const speed = playbackSpeeds.find((speed) => String(speed) === value);
    if (speed !== undefined) onChange(speed);
  }
  return (
    <div data-studio-preserve-selection>
      <Select value={String(value)} onValueChange={change}>
        <SelectTrigger aria-label="Playback speed" className="w-20">
          <SelectValue />
        </SelectTrigger>
        <SelectContent data-studio-preserve-selection>
          {playbackSpeeds.map((speed) => (
            <SelectItem key={speed} value={String(speed)}>
              {speed}x
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
}
