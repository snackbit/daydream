import { useCallback, useRef, useState } from 'react';
import type { OcrDocument } from '@daydream/shared';
import { Copy, Expand, Search } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogDescription,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
  TooltipProvider,
} from '@/components/ui/tooltip';
import { OcrReadableText } from './ocr-readable-text';
import { ocrMatchCount, uncertainOcrWordCount } from './ocr-text-model';

export interface OcrTextPanelProps {
  document: OcrDocument;
}
export function OcrTextPanel({ document }: OcrTextPanelProps) {
  const [query, setQuery] = useState('');
  const readingRef = useRef<HTMLDivElement | null>(null);
  const focusReadingText = useCallback((event: Event) => {
    event.preventDefault();
    readingRef.current?.focus();
  }, []);
  const [copyStatus, setCopyStatus] = useState<'idle' | 'copied' | 'error'>(
    'idle',
  );
  const matches = ocrMatchCount(document.text, query);
  const uncertain = uncertainOcrWordCount(document);
  const copy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(document.text);
      setCopyStatus('copied');
    } catch {
      setCopyStatus('error');
    }
  }, [document.text]);
  const changeQuery = useCallback(
    (event: React.ChangeEvent<HTMLInputElement>) =>
      setQuery(event.currentTarget.value),
    [],
  );
  return (
    <TooltipProvider delayDuration={200}>
      <div className="space-y-2">
        {document.provenance === 'reconstructed' ? (
          <p className="text-muted-foreground text-[10px]">
            Reconstructed from legacy OCR regions. Reading order is approximate.
          </p>
        ) : null}
        <div className="flex gap-1">
          <div className="relative min-w-0 flex-1">
            <Search
              aria-hidden
              className="text-muted-foreground absolute top-1/2 left-2 size-3 -translate-y-1/2"
            />
            <Input
              aria-label="Find OCR text"
              className="h-7 pl-6"
              onChange={changeQuery}
              placeholder="Find text…"
              value={query}
            />
          </div>
          <Button
            aria-label="Copy OCR text"
            onClick={copy}
            size="icon-sm"
            variant="outline"
          >
            <Copy aria-hidden className="size-3" />
          </Button>
          <Dialog>
            <DialogTrigger asChild>
              <Button
                aria-label="Expand OCR text"
                size="icon-sm"
                variant="outline"
              >
                <Expand aria-hidden className="size-3" />
              </Button>
            </DialogTrigger>
            <DialogContent
              className="flex h-[85vh] max-h-[85vh] max-w-3xl flex-col overflow-hidden"
              onOpenAutoFocus={focusReadingText}
            >
              <DialogHeader className="shrink-0">
                <DialogTitle>OCR reading view</DialogTitle>
                <DialogDescription>
                  {document.provenance === 'reconstructed'
                    ? 'Literal text reconstructed from legacy OCR regions; reading order is approximate.'
                    : 'Literal recognized text from the selected source frame.'}
                </DialogDescription>
              </DialogHeader>
              <ScrollArea
                className="min-h-0 flex-1 overflow-hidden"
                data-testid="ocr-reading-scroll"
              >
                <div
                  aria-label="OCR reading text"
                  ref={readingRef}
                  tabIndex={-1}
                >
                  <OcrReadableText document={document} query={query} />
                </div>
              </ScrollArea>
            </DialogContent>
          </Dialog>
        </div>
        <div
          className="text-muted-foreground flex flex-wrap gap-x-3 text-[10px]"
          aria-live="polite"
        >
          {query.trim() ? (
            <span>
              {matches} {matches === 1 ? 'match' : 'matches'}
            </span>
          ) : null}
          {uncertain > 0 ? (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  aria-label="Uncertain OCR words"
                  className="h-auto px-0 py-0 text-[10px]"
                  size="sm"
                  variant="ghost"
                >
                  {uncertain} uncertain {uncertain === 1 ? 'word' : 'words'}{' '}
                  retained
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                Tesseract scored these words below 0.5. Their original text
                remains visible and copyable.
              </TooltipContent>
            </Tooltip>
          ) : null}
          {document.engine === 'vision' &&
          document.blocks.every((block) => block.confidence === null) ? (
            <span>Confidence unknown</span>
          ) : null}
          {copyStatus === 'copied' ? (
            <span>Text copied</span>
          ) : copyStatus === 'error' ? (
            <span>Copy failed</span>
          ) : null}
        </div>
        <OcrReadableText document={document} query={query} />
      </div>
    </TooltipProvider>
  );
}
