import { useEffect } from "react";
import { useScanStore } from "../store/scanStore";

/**
 * Convenience hook over the scan store. Loads volumes once on mount and
 * exposes the full scan surface (state + actions) for views.
 */
export function useScan() {
  const volumes = useScanStore((s) => s.volumes);
  const volumesLoading = useScanStore((s) => s.volumesLoading);
  const result = useScanStore((s) => s.result);
  const status = useScanStore((s) => s.status);
  const progress = useScanStore((s) => s.progress);
  const target = useScanStore((s) => s.target);
  const error = useScanStore((s) => s.error);

  const loadVolumes = useScanStore((s) => s.loadVolumes);
  const scan = useScanStore((s) => s.scan);
  const cancel = useScanStore((s) => s.cancel);
  const reset = useScanStore((s) => s.reset);

  useEffect(() => {
    if (volumes.length === 0 && !volumesLoading) {
      void loadVolumes();
    }
    // Run once on mount; loadVolumes is stable from zustand.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return {
    volumes,
    volumesLoading,
    result,
    status,
    progress,
    target,
    error,
    loadVolumes,
    scan,
    cancel,
    reset,
  };
}
