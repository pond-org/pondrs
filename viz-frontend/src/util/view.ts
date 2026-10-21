import { useCallback, useEffect, useState } from 'react';

/** How the canvas is drawn; set from the header's view menu. */
export interface ViewOptions {
  theme: 'dark' | 'light';
  /** Draw expanded groups as boxes around their children. */
  boxes: boolean;
  /** Show datasets passed between steps; off = an edge from producer to consumer. */
  datasets: boolean;
  /** Show params. */
  params: boolean;
}

export const DEFAULT_VIEW: ViewOptions = { theme: 'dark', boxes: true, datasets: true, params: true };

const VIEW_KEY = 'pondrs-viz.view';

/**
 * View options, remembered per browser. Saved values are spread over the
 * defaults, so an option added later starts at its default.
 */
export function useViewOptions() {
  const [view, setView] = useState<ViewOptions>(() => {
    try {
      return { ...DEFAULT_VIEW, ...JSON.parse(localStorage.getItem(VIEW_KEY) ?? '{}') };
    } catch {
      return DEFAULT_VIEW;
    }
  });

  useEffect(() => {
    try {
      localStorage.setItem(VIEW_KEY, JSON.stringify(view));
    } catch {
      // Storage unavailable (private window, blocked site data): not remembered.
    }
  }, [view]);

  const setOption = useCallback(<K extends keyof ViewOptions>(key: K, value: ViewOptions[K]) => {
    setView(prev => ({ ...prev, [key]: value }));
  }, []);

  return [view, setOption] as const;
}
