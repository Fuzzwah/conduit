import { FileEdit } from 'lucide-react';
import { ToolCard, type ToolStatus } from './ToolCard';
import { DiffViewer } from '../DiffViewer';
import { isDiffContent } from '../../lib/diffParser';
import { FilePathLink } from '../FilePathLink';
import { CodeBlock } from '../markdown/CodeBlock';

interface EditToolCardProps {
  status: ToolStatus;
  filePath: string;
  content?: string;
  error?: string;
}

export function EditToolCard({ status, filePath, content, error }: EditToolCardProps) {
  const hasDiff = content && isDiffContent(content);

  return (
    <ToolCard
      icon={<FileEdit className="h-4 w-4" />}
      title="Edit"
      subtitle={<FilePathLink path={filePath} className="text-xs" />}
      status={status}
    >
      {error ? (
        <div className="p-3 text-sm text-error">{error}</div>
      ) : content ? (
        <div className="p-2">
          {hasDiff ? (
            <DiffViewer diff={content} />
          ) : (
            <CodeBlock code={content} surface="sourceFile" />
          )}
        </div>
      ) : null}
    </ToolCard>
  );
}
