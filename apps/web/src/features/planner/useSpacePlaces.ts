import { useEffect, useRef, useState } from "react";
import { listSavedPlaces, PlacesApiError } from "./placesApi";

type PlacePage = Awaited<ReturnType<typeof listSavedPlaces>>;
type Scope = { category: string | null; revision: number; retry: number };
type Result =
  | { scope: Scope; page: PlacePage }
  | { scope: Scope; error: string };

export function useSpacePlaces(
  accessToken: string,
  spaceId: string,
  revision: number,
) {
  const [category, setCategory] = useState<string | null>(null);
  const [retry, setRetry] = useState(0);
  const [result, setResult] = useState<Result>();
  const [counts, setCounts] = useState<PlacePage["counts"]>();
  const [pageError, setPageError] = useState("");
  const [loadingMore, setLoadingMore] = useState(false);
  const request = useRef<AbortController | null>(null);
  const paging = useRef(false);
  const current =
    result?.scope.category === category &&
    result.scope.revision === revision &&
    result.scope.retry === retry
      ? result
      : undefined;
  const page = current && "page" in current ? current.page : undefined;

  useEffect(() => {
    if (!spaceId) return;
    const controller = new AbortController();
    request.current = controller;
    paging.current = false;
    setLoadingMore(false);
    setPageError("");
    const scope = { category, revision, retry };
    listSavedPlaces(accessToken, spaceId, category, 0, controller.signal)
      .then((page) => {
        if (controller.signal.aborted) return;
        setResult({ scope, page });
        setCounts(page.counts);
      })
      .catch((error: unknown) => {
        if (controller.signal.aborted) return;
        setResult({
          scope,
          error:
            error instanceof PlacesApiError && error.status === 403
              ? "You don’t have access to Saved places in this Space."
              : "Saved places couldn’t be loaded. Check your connection and retry.",
        });
      });
    return () => controller.abort();
  }, [accessToken, spaceId, category, revision, retry]);

  const loadMore = async () => {
    const signal = request.current?.signal;
    if (
      !signal ||
      signal.aborted ||
      !page ||
      page.nextOffset === null ||
      paging.current
    )
      return;
    paging.current = true;
    setLoadingMore(true);
    setPageError("");
    try {
      const next = await listSavedPlaces(
        accessToken,
        spaceId,
        category,
        page.nextOffset,
        signal,
      );
      if (signal.aborted) return;
      const seen = new Set(page.items.map((item) => item.id));
      setResult({
        scope: { category, revision, retry },
        page: {
          ...next,
          items: [
            ...page.items,
            ...next.items.filter((item) => !seen.has(item.id)),
          ],
        },
      });
      setCounts(next.counts);
    } catch {
      if (!signal.aborted)
        setPageError("More Saved places couldn’t be loaded. Please retry.");
    } finally {
      if (!signal.aborted) {
        paging.current = false;
        setLoadingMore(false);
      }
    }
  };

  return {
    category,
    setCategory,
    counts,
    page,
    error: current && "error" in current ? current.error : "",
    retry: () => setRetry((value) => value + 1),
    pageError,
    loadingMore,
    loadMore,
  };
}
