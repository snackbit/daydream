import { useState, type ChangeEvent } from 'react';
import type { RecallSession } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { EvidenceSourceLink } from '../components/evidence-source-link';
import { setRecallSessionCorrection } from '../recall-client';
import { recallTime } from '../recall-format';

export interface DaySessionProps {
  session: RecallSession;
  dayId: string;
  busy: boolean;
  onAction: (action: () => Promise<unknown>) => void;
}
export function DaySession({
  session,
  dayId,
  busy,
  onAction,
}: DaySessionProps) {
  const [editing, setEditing] = useState(false);
  const [correction, setCorrection] = useState(session.correction ?? '');
  const [titleCorrection, setTitleCorrection] = useState(
    session.titleCorrection ?? session.title,
  );
  const [categoryCorrection, setCategoryCorrection] = useState(
    session.categoryCorrection ?? session.category,
  );
  function changeTitle(event: ChangeEvent<HTMLInputElement>) {
    setTitleCorrection(event.target.value);
  }
  function changeCategory(event: ChangeEvent<HTMLInputElement>) {
    setCategoryCorrection(event.target.value);
  }
  function toggleEditing() {
    setEditing((value) => !value);
  }
  function changeCorrection(event: ChangeEvent<HTMLInputElement>) {
    setCorrection(event.target.value);
  }
  function save() {
    onAction(() =>
      setRecallSessionCorrection({
        dayId,
        sessionId: session.id,
        correction: correction.trim() || null,
        titleCorrection:
          titleCorrection.trim() === session.title
            ? null
            : titleCorrection.trim() || null,
        categoryCorrection:
          categoryCorrection.trim() === session.category
            ? null
            : categoryCorrection.trim() || null,
      }),
    );
  }
  return (
    <article className="border-border grid gap-3 border-b py-6">
      <p className="text-muted-foreground text-xs">
        {recallTime(session.startMs)}–{recallTime(session.endMs)}
      </p>
      <h3 className="text-base font-medium break-words">
        {session.titleCorrection ?? session.title}
      </h3>
      <p className="text-muted-foreground text-xs">
        {session.categoryCorrection ?? session.category}
      </p>
      <p className="text-muted-foreground text-sm leading-6 break-words">
        {session.summary}
      </p>
      {session.correction ? (
        <p className="text-sm">Your correction: {session.correction}</p>
      ) : null}
      <div className="flex flex-wrap gap-4">
        {session.citations.map((citation) => (
          <EvidenceSourceLink key={citation.evidenceId} citation={citation} />
        ))}
      </div>
      <Button
        className="w-fit"
        onClick={toggleEditing}
        disabled={busy}
        variant="ghost"
        aria-expanded={editing}
      >
        Correct this session
      </Button>
      {editing ? (
        <div className="grid gap-3">
          <Label htmlFor={`title-${session.id}`}>Session title</Label>
          <Input
            id={`title-${session.id}`}
            disabled={busy}
            value={titleCorrection}
            onChange={changeTitle}
          />
          <Label htmlFor={`category-${session.id}`}>Session category</Label>
          <Input
            id={`category-${session.id}`}
            disabled={busy}
            value={categoryCorrection}
            onChange={changeCategory}
          />
          <Label htmlFor={`correction-${session.id}`}>Local correction</Label>
          <Input
            id={`correction-${session.id}`}
            value={correction}
            disabled={busy}
            onChange={changeCorrection}
          />
          <Button
            className="w-fit"
            disabled={busy}
            onClick={save}
            variant="outline"
          >
            Save correction
          </Button>
        </div>
      ) : null}
    </article>
  );
}
