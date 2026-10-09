import { useCallback, useEffect, useState } from 'react';
import { AppWindow } from 'lucide-react';
import { getWindowApplicationIcon } from './window-icon-client';

export interface WindowApplicationIconProps {
  iconKey: string | null | undefined;
}
export function WindowApplicationIcon({ iconKey }: WindowApplicationIconProps) {
  const [image, setImage] = useState<{ key: string; url: string } | null>(null);
  const handleImageError = useCallback(() => setImage(null), []);
  useEffect(() => {
    if (!iconKey) return;
    let active = true;
    void getWindowApplicationIcon(iconKey)
      .then((url) => {
        if (active && url) setImage({ key: iconKey, url });
      })
      .catch(() => undefined);
    return () => {
      active = false;
    };
  }, [iconKey]);
  return image && image.key === iconKey ? (
    <img
      alt=""
      aria-hidden="true"
      src={image.url}
      onError={handleImageError}
      width={32}
      height={32}
      className="size-8 shrink-0 object-contain"
    />
  ) : (
    <AppWindow
      aria-hidden="true"
      className="text-muted-foreground size-8 shrink-0"
    />
  );
}
