import * as React from 'react';
import { Slider as SliderPrimitive } from 'radix-ui';
import { cn } from '@/lib/utils';

function Slider({
  'aria-label': ariaLabel,
  'aria-labelledby': ariaLabelledBy,
  className,
  defaultValue,
  max = 100,
  min = 0,
  value,
  ...props
}: React.ComponentProps<typeof SliderPrimitive.Root>) {
  const _values = React.useMemo(
    () =>
      Array.isArray(value)
        ? value
        : Array.isArray(defaultValue)
          ? defaultValue
          : [min, max],
    [value, defaultValue, min, max],
  );

  return (
    <SliderPrimitive.Root
      data-slot="slider"
      max={max}
      min={min}
      className={cn(
        'relative flex w-full touch-none items-center select-none data-disabled:opacity-50 data-vertical:h-full data-vertical:min-h-40 data-vertical:w-auto data-vertical:flex-col',
        className,
      )}
      {...(defaultValue === undefined ? {} : { defaultValue })}
      {...(value === undefined ? {} : { value })}
      {...props}
    >
      <SliderPrimitive.Track
        data-slot="slider-track"
        className="bg-muted relative grow overflow-hidden rounded-md data-horizontal:h-1 data-horizontal:w-full data-vertical:h-full data-vertical:w-1"
      >
        <SliderPrimitive.Range
          data-slot="slider-range"
          className="bg-primary absolute select-none data-horizontal:h-full data-vertical:w-full"
        />
      </SliderPrimitive.Track>
      {Array.from({ length: _values.length }, (_, index) => (
        <SliderPrimitive.Thumb
          aria-label={ariaLabel}
          aria-labelledby={ariaLabelledBy}
          data-slot="slider-thumb"
          key={index}
          className="border-ring ring-ring/30 relative block size-3 shrink-0 rounded-md border bg-white transition-[color,box-shadow] select-none after:absolute after:-inset-2 hover:ring-2 focus-visible:ring-2 focus-visible:outline-hidden active:ring-2 disabled:pointer-events-none disabled:opacity-50"
        />
      ))}
    </SliderPrimitive.Root>
  );
}

export { Slider };
