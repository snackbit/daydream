import { Link } from '@tanstack/react-router';
import type { RecallCitation } from '@daydream/shared';
import { ArrowUpRight } from 'lucide-react';
import { recallTime } from '../recall-format';

export interface EvidenceSourceLinkProps {
  citation: RecallCitation;
}
export function EvidenceSourceLink({ citation }: EvidenceSourceLinkProps) {
  return (
    <Link
      className="inline-flex items-center gap-1 text-xs underline-offset-4 hover:underline"
      to="/library/$recordingId"
      params={{ recordingId: citation.recordingId }}
      search={{
        evidenceId: citation.evidenceId,
        source: citation.sourceKind,
        seek: null,
      }}
    >
      {citation.sourceKind === 'ocr'
        ? 'Screen text'
        : citation.sourceKind === 'desktop-audio'
          ? 'Desktop audio'
          : 'Microphone'}{' '}
      · {recallTime(citation.absoluteStartMs)}{' '}
      <ArrowUpRight aria-hidden="true" className="size-3" />
    </Link>
  );
}
