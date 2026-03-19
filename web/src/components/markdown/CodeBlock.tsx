import { useState, useEffect, useMemo, useRef } from 'react';
import { ChevronDown, ChevronUp } from 'lucide-react';
import DOMPurify from 'dompurify';
import { cn } from '../../lib/cn';
import { CopyButton } from '../ui/CopyButton';
import { highlightCodeToHtml, normalizeLanguage, type CodeSurface } from '../../lib/shiki';
import { useTheme } from '../../hooks/useTheme';

interface CodeBlockProps {
  code: string;
  language?: string;
  surface: Exclude<CodeSurface, 'markdownInline'>;
  showLineNumbers?: boolean;
  maxHeight?: number;
  className?: string;
}

const COLLAPSE_THRESHOLD = 20; // lines
const DEFAULT_MAX_HEIGHT = 400; // px

export function CodeBlock({
  code,
  language,
  surface,
  showLineNumbers = false,
  maxHeight = DEFAULT_MAX_HEIGHT,
  className,
}: CodeBlockProps) {
  const { currentTheme } = useTheme();
  const [highlightedHtml, setHighlightedHtml] = useState<string | null>(null);
  const [isExpanded, setIsExpanded] = useState(false);
  const [shouldCollapse, setShouldCollapse] = useState(false);
  const contentRef = useRef<HTMLDivElement>(null);
  const sanitizedHighlightedHtml = useMemo(
    () => (highlightedHtml ? DOMPurify.sanitize(highlightedHtml) : null),
    [highlightedHtml]
  );

  const normalizedLang = normalizeLanguage(language);
  const lineCount = code.split('\n').length;

  useEffect(() => {
    let cancelled = false;

    async function highlight() {
      try {
        if (!currentTheme) return;
        const html = await highlightCodeToHtml(code, normalizedLang, surface, currentTheme);

        if (!cancelled) {
          setHighlightedHtml(html);
        }
      } catch (err) {
        console.error('Syntax highlighting failed:', err);
        // Fallback to plain text
        if (!cancelled) {
          setHighlightedHtml(null);
        }
      }
    }

    highlight();
    return () => {
      cancelled = true;
    };
  }, [code, currentTheme, normalizedLang, surface]);

  useEffect(() => {
    if (contentRef.current) {
      const scrollHeight = contentRef.current.scrollHeight;
      setShouldCollapse(lineCount > COLLAPSE_THRESHOLD || scrollHeight > maxHeight);
    }
  }, [highlightedHtml, lineCount, maxHeight]);

  const displayLanguage = normalizedLang !== 'text' ? normalizedLang : null;

  return (
    <div className={cn('group relative rounded-lg border border-border overflow-hidden', className)}>
      {/* Header with language badge and copy button */}
      <div className="flex items-center justify-between bg-surface-elevated px-3 py-1.5 border-b border-border">
        <span className="text-xs font-medium text-text-muted">
          {displayLanguage || 'text'}
        </span>
        <CopyButton text={code} />
      </div>

      {/* Code content */}
      <div
        ref={contentRef}
        className={cn(
          'overflow-auto',
          surface === 'sourceFile' && 'bg-background',
          !isExpanded && shouldCollapse && 'max-h-[400px]'
        )}
        style={!isExpanded && shouldCollapse ? { maxHeight } : undefined}
      >
        {sanitizedHighlightedHtml ? (
          <div
            className={cn(
              'shiki-wrapper p-3',
              showLineNumbers && 'show-line-numbers'
            )}
            dangerouslySetInnerHTML={{ __html: sanitizedHighlightedHtml }}
          />
        ) : (
          <pre className="p-3 text-sm text-text overflow-x-auto">
            <code>{code}</code>
          </pre>
        )}
      </div>

      {/* Expand/collapse button */}
      {shouldCollapse && (
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          className={cn(
            'flex w-full items-center justify-center gap-1 py-1.5',
            'bg-surface-elevated border-t border-border',
            'text-xs text-text-muted hover:text-text',
            'transition-colors duration-150'
          )}
        >
          {isExpanded ? (
            <>
              <ChevronUp className="h-3.5 w-3.5" />
              <span>Collapse</span>
            </>
          ) : (
            <>
              <ChevronDown className="h-3.5 w-3.5" />
              <span>Expand ({lineCount} lines)</span>
            </>
          )}
        </button>
      )}
    </div>
  );
}
