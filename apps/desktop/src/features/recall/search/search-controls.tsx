import { useState, type ChangeEvent, type FormEvent } from 'react';
import type { RecallSourceKind } from '@daydream/shared';
import { Search } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';

/** URL-addressable query intent; pagination remains owned by the current query. */
export interface RecallSearchLocation {
  q: string;
  source: RecallSourceKind | 'all';
  date: string | null;
}
export interface SearchControlsProps {
  location: RecallSearchLocation;
  onSearchChange: (search: RecallSearchLocation) => void;
  mode: 'hybrid' | 'lexical';
  onModeChange: (mode: 'hybrid' | 'lexical') => void;
}
export function SearchControls({
  location,
  onSearchChange,
  mode,
  onModeChange,
}: SearchControlsProps) {
  const [query, setQuery] = useState(location.q);
  const [date, setDate] = useState(location.date ?? '');
  const [source, setSource] = useState(location.source);
  function changeQuery(event: ChangeEvent<HTMLInputElement>) {
    setQuery(event.target.value);
  }
  function changeDate(event: ChangeEvent<HTMLInputElement>) {
    setDate(event.target.value);
  }
  function changeSource(value: string) {
    if (
      value === 'all' ||
      value === 'ocr' ||
      value === 'desktop-audio' ||
      value === 'microphone'
    )
      setSource(value);
  }
  function changeMode(value: string) {
    if (value === 'hybrid' || value === 'lexical') onModeChange(value);
  }
  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    onSearchChange({ q: query.trim(), source, date: date || null });
  }
  return (
    <form
      onSubmit={submit}
      className="bg-muted grid gap-5 rounded-[24px] p-6 sm:p-8"
    >
      <div className="grid gap-2">
        <Label htmlFor="recall-query">Find a moment</Label>
        <div className="flex flex-wrap gap-3">
          <Input
            id="recall-query"
            className="min-w-0 flex-1 basis-48"
            placeholder="Words, ideas, or something you remember…"
            value={query}
            onChange={changeQuery}
          />
          <Button disabled={!query.trim()} type="submit">
            <Search aria-hidden="true" />
            Search
          </Button>
        </div>
      </div>
      <div className="grid gap-4 sm:grid-cols-3">
        <div className="grid gap-2">
          <Label htmlFor="recall-source">Source</Label>
          <Select value={source} onValueChange={changeSource}>
            <SelectTrigger id="recall-source" className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All sources</SelectItem>
              <SelectItem value="ocr">Screen text</SelectItem>
              <SelectItem value="desktop-audio">Desktop audio</SelectItem>
              <SelectItem value="microphone">Microphone</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div className="grid gap-2">
          <Label htmlFor="recall-date">Day</Label>
          <Input
            id="recall-date"
            type="date"
            value={date}
            onChange={changeDate}
          />
        </div>
        <div className="grid gap-2">
          <Label htmlFor="recall-mode">Match</Label>
          <Select value={mode} onValueChange={changeMode}>
            <SelectTrigger id="recall-mode" className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="hybrid">Semantic + literal</SelectItem>
              <SelectItem value="lexical">Literal words</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>
      <p className="text-muted-foreground text-xs">
        Search only eligible screen text and transcripts. Source and date
        filters apply when you search.
      </p>
    </form>
  );
}
