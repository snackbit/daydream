import { useState, type ChangeEvent } from 'react';
import type { RecallTask } from '@daydream/shared';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { EvidenceSourceLink } from '../components/evidence-source-link';
import { updateRecallTask } from '../recall-client';

export interface DayTaskProps {
  task: RecallTask;
  busy: boolean;
  onAction: (action: () => Promise<unknown>) => void;
}
export function DayTask({ task, busy, onAction }: DayTaskProps) {
  const [editing, setEditing] = useState(false);
  const [title, setTitle] = useState(task.sourceWithheld ? '' : task.title);
  const [notes, setNotes] = useState(task.sourceWithheld ? '' : task.notes);
  function changeTitle(event: ChangeEvent<HTMLInputElement>) {
    setTitle(event.target.value);
  }
  function changeNotes(event: ChangeEvent<HTMLInputElement>) {
    setNotes(event.target.value);
  }
  function edit() {
    setEditing((value) => !value);
  }
  function save() {
    onAction(() =>
      updateRecallTask({ taskId: task.id, title, notes, status: task.status }),
    );
  }
  function setStatus(status: RecallTask['status']) {
    onAction(() =>
      updateRecallTask({
        taskId: task.id,
        title: task.title,
        notes: task.notes,
        status,
      }),
    );
  }
  function done() {
    setStatus('done');
  }
  function reopen() {
    setStatus('open');
  }
  function dismiss() {
    setStatus('dismissed');
  }
  return (
    <article className="border-border grid gap-3 border-t py-5">
      <Badge className="w-fit" variant="outline">
        {task.status}
      </Badge>
      {task.sourceWithheld ? (
        <p className="text-muted-foreground text-sm">
          Source content withheld after a privacy change. Your decision is
          preserved.
        </p>
      ) : (
        <>
          <h3 className="text-sm font-medium break-words">{task.title}</h3>
          <p className="text-muted-foreground text-sm break-words">
            {task.notes}
          </p>
          <div className="flex flex-wrap gap-3">
            {task.citations.map((citation) => (
              <EvidenceSourceLink
                key={citation.evidenceId}
                citation={citation}
              />
            ))}
          </div>
          <div className="flex flex-wrap gap-2">
            <Button disabled={busy} onClick={edit} variant="ghost">
              Edit
            </Button>
            <Button
              disabled={busy}
              onClick={task.status === 'open' ? done : reopen}
              variant="outline"
            >
              {task.status === 'open' ? 'Mark done' : 'Reopen'}
            </Button>
            <Button
              disabled={busy || task.status === 'dismissed'}
              onClick={dismiss}
              variant="ghost"
            >
              Dismiss
            </Button>
          </div>
        </>
      )}
      {editing && !task.sourceWithheld ? (
        <div className="grid gap-3">
          <Label htmlFor={`task-title-${task.id}`}>Task title</Label>
          <Input
            id={`task-title-${task.id}`}
            disabled={busy}
            value={title}
            onChange={changeTitle}
          />
          <Label htmlFor={`task-notes-${task.id}`}>Notes</Label>
          <Input
            id={`task-notes-${task.id}`}
            disabled={busy}
            value={notes}
            onChange={changeNotes}
          />
          <Button
            disabled={busy || !title.trim()}
            onClick={save}
            variant="outline"
          >
            Save task
          </Button>
        </div>
      ) : null}
    </article>
  );
}
