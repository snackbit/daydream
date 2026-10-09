import { useCallback, type ChangeEvent } from 'react';
import type { RecorderSceneSource } from '@daydream/shared';
import { SharedDisclosure } from '@/components/shared-disclosure';
import { Button } from '@/components/ui/button';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  cropSource,
  croppedDimensions,
  resizeSource,
} from '../source-capture-geometry';

export interface SourceCapturePropertiesProps {
  source: RecorderSceneSource;
  disabled: boolean;
  onChange: (source: RecorderSceneSource) => void;
  regionId: string | null;
  customId: string | null;
  onRegionChange: (sourceId: string | null) => void;
  onCustomChange: (sourceId: string | null) => void;
}

export function SourceCaptureProperties({
  source,
  disabled,
  onChange,
  regionId,
  customId,
  onRegionChange: setRegionId,
  onCustomChange: setCustomId,
}: SourceCapturePropertiesProps) {
  const crop = croppedDimensions(source);
  const region =
    regionId === source.id ||
    Object.values(source.crop).some((value) => value !== 0);
  const scale = crop.width > 0 ? source.rect.width / crop.width : 1;
  const scaleChoice =
    customId === source.id
      ? 'custom'
      : Math.abs(scale - 1) < 0.01
        ? '100'
        : Math.abs(scale - 0.5) < 0.01
          ? '50'
          : 'custom';
  function changeCrop(field: 'x' | 'y' | 'width' | 'height', value: number) {
    onChange(
      cropSource(source, {
        x: source.crop.left,
        y: source.crop.top,
        width: crop.width,
        height: crop.height,
        [field]: value,
      }),
    );
  }
  function setCrop(value: string) {
    if (value === 'full') {
      setRegionId(null);
      onChange(
        cropSource(source, {
          x: 0,
          y: 0,
          width: source.sourceWidth ?? 0,
          height: source.sourceHeight ?? 0,
        }),
      );
    } else if (value === 'region') setRegionId(source.id);
  }
  function setScale(value: string) {
    setCustomId(null);
    if (value === '100' || value === '50')
      onChange(resizeSource(source, Number(value) / 100));
  }
  function setOutputWidth(width: number) {
    onChange(resizeSource(source, width / crop.width));
  }
  function changeCropX(value: number) {
    changeCrop('x', value);
  }
  function changeCropY(value: number) {
    changeCrop('y', value);
  }
  function changeCropWidth(value: number) {
    changeCrop('width', value);
  }
  function changeCropHeight(value: number) {
    changeCrop('height', value);
  }
  function changeScale(value: string) {
    if (value === 'custom') setCustomId(source.id);
    else setScale(value);
  }
  const reset = useCallback(() => {
    setRegionId(null);
    setCustomId(null);
    onChange(
      resizeSource(
        { ...source, crop: { left: 0, top: 0, right: 0, bottom: 0 } },
        1,
      ),
    );
  }, [source, onChange, setRegionId, setCustomId]);
  return (
    <div className="space-y-6 p-6 text-sm">
      <div>
        <h2 className="text-sm font-medium">Source output</h2>
        <p className="text-muted-foreground mt-1 text-xs">
          {source.sourceWidth} × {source.sourceHeight} pixels
        </p>
      </div>
      <Field>
        <FieldLabel htmlFor="source-crop-mode">Capture area</FieldLabel>
        <Select
          disabled={disabled}
          value={region ? 'region' : 'full'}
          onValueChange={setCrop}
        >
          <SelectTrigger id="source-crop-mode">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="full">Full source</SelectItem>
            <SelectItem value="region">Region</SelectItem>
          </SelectContent>
        </Select>
      </Field>
      {region ? (
        <SharedDisclosure label="Precision crop controls">
          <div className="grid grid-cols-2 gap-2">
            <SourceNumberField
              label="Crop X"
              value={source.crop.left}
              min={0}
              disabled={disabled}
              onChange={changeCropX}
            />
            <SourceNumberField
              label="Crop Y"
              value={source.crop.top}
              min={0}
              disabled={disabled}
              onChange={changeCropY}
            />
            <SourceNumberField
              label="Crop width"
              value={crop.width}
              min={2}
              disabled={disabled}
              onChange={changeCropWidth}
            />
            <SourceNumberField
              label="Crop height"
              value={crop.height}
              min={2}
              disabled={disabled}
              onChange={changeCropHeight}
            />
          </div>
        </SharedDisclosure>
      ) : null}
      <Field>
        <FieldLabel htmlFor="source-scale">Output scale</FieldLabel>
        <Select
          disabled={disabled}
          value={scaleChoice}
          onValueChange={changeScale}
        >
          <SelectTrigger id="source-scale">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="100">100%</SelectItem>
            <SelectItem value="50">50%</SelectItem>
            <SelectItem value="custom">Custom size</SelectItem>
          </SelectContent>
        </Select>
      </Field>
      {scaleChoice === 'custom' ? (
        <SourceNumberField
          label="Output width"
          value={source.rect.width}
          min={2}
          disabled={disabled}
          onChange={setOutputWidth}
        />
      ) : null}
      <p className="text-xs tabular-nums">
        Saved video: {source.rect.width} × {source.rect.height}
      </p>
      <p className="text-muted-foreground text-xs leading-5">
        Crop first, then resize. Recording and AI use this output.
      </p>
      <Button disabled={disabled} size="sm" variant="outline" onClick={reset}>
        Reset crop and scale
      </Button>
    </div>
  );
}

function SourceNumberField({
  label,
  value,
  min,
  disabled,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  disabled: boolean;
  onChange: (value: number) => void;
}) {
  const changeValue = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      const next = Number(event.target.value);
      if (Number.isFinite(next) && Number.isInteger(next)) onChange(next);
    },
    [onChange],
  );
  return (
    <Field>
      <FieldLabel htmlFor={label}>{label}</FieldLabel>
      <Input
        id={label}
        type="number"
        min={min}
        step={1}
        disabled={disabled}
        value={value}
        onChange={changeValue}
      />
    </Field>
  );
}
