import { useMemo } from 'react';
import type { OcrDocument } from '@daydream/shared';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
  TooltipProvider,
} from '@/components/ui/tooltip';
import { ocrTextPieces } from './ocr-text-model';

export interface OcrReadableTextProps {
  document: OcrDocument;
  query: string;
}
export function OcrReadableText({ document, query }: OcrReadableTextProps) {
  const pieces = useMemo(
    () => ocrTextPieces(document, query),
    [document, query],
  );
  return (
    <TooltipProvider delayDuration={200}>
      <p
        className="text-[11px] leading-5 break-words whitespace-pre-wrap"
        aria-label="Recognized text"
      >
        {pieces.map((piece, index) => {
          const content = piece.match ? (
            <mark className="bg-amber-200 text-neutral-950">{piece.text}</mark>
          ) : (
            piece.text
          );
          return piece.uncertain ? (
            <Tooltip key={index}>
              <TooltipTrigger asChild>
                <span className="decoration-muted-foreground underline decoration-dotted underline-offset-4">
                  {content}
                </span>
              </TooltipTrigger>
              <TooltipContent>
                Uncertain OCR word; original text retained.
              </TooltipContent>
            </Tooltip>
          ) : (
            <span key={index}>{content}</span>
          );
        })}
      </p>
    </TooltipProvider>
  );
}
