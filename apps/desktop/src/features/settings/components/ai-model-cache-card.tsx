import type { AiModelAsset } from '@daydream/shared';
import { CheckCircle2, HardDriveDownload } from 'lucide-react';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import {
  ModelTransferRow,
  type ModelTransferRowProps,
} from '@/features/ai/models/model-transfer-row';

export interface AiModelCacheCardProps {
  models: AiModelAsset[];
  onCancelModelDownload: (modelId: string) => void;
  onDeleteModel: (modelId: string) => void;
  onDownloadModel: (modelId: string) => void;
  modelActionErrors?: Readonly<Record<string, string>> | undefined;
  onTransferAction?: ModelTransferRowProps['onTransferAction'];
}
export function AiModelCacheCard({
  models,
  modelActionErrors,
  ...actions
}: AiModelCacheCardProps) {
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>Whisper Model Cache</CardTitle>
        <CardDescription>
          Downloads are verified before models become ready.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="divide-y">
          {models.map((model) => (
            <ModelTransferRow
              key={model.id}
              model={model}
              actionError={modelActionErrors?.[model.id]}
              {...actions}
            />
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
export type ModelCacheRowProps = ModelTransferRowProps;
export const ModelCacheRow = ModelTransferRow;
export function AiModelActionButton({
  model,
  ...actions
}: Omit<ModelTransferRowProps, 'model'> & { model: AiModelAsset | undefined }) {
  return model ? <ModelTransferRow model={model} {...actions} /> : null;
}
export function ModelStatusIcon({ model }: { model: AiModelAsset }) {
  return model.downloadStatus === 'available' ? (
    <CheckCircle2 aria-hidden="true" className="size-4 shrink-0" />
  ) : (
    <HardDriveDownload aria-hidden="true" className="size-4 shrink-0" />
  );
}
