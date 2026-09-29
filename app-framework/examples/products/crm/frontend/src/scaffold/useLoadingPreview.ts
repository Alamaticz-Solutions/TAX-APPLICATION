import { useEffect, useRef, useState } from "react";

export type LoadingPreviewOptions = {
  delayMs?: number;
  minVisibleMs?: number;
};

export function useLoadingPreview(
  active: boolean,
  { delayMs = 140, minVisibleMs = 260 }: LoadingPreviewOptions = {}
) {
  const [visible, setVisible] = useState(false);
  const shownAtRef = useRef(0);

  useEffect(() => {
    let timer: number | undefined;

    if (active) {
      if (!visible) {
        timer = window.setTimeout(() => {
          shownAtRef.current = Date.now();
          setVisible(true);
        }, delayMs);
      }
    } else if (visible) {
      const elapsed = Date.now() - shownAtRef.current;
      timer = window.setTimeout(() => setVisible(false), Math.max(0, minVisibleMs - elapsed));
    }

    return () => {
      if (timer !== undefined) window.clearTimeout(timer);
    };
  }, [active, delayMs, minVisibleMs, visible]);

  return visible;
}
