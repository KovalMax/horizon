import { useEffect, useState } from "react";
import type { EventItem, Space } from "./domain";
import { listCalendarEvents, nextCalendarEvent } from "./eventsApi";

const initialDays = 60;

function localDate(offset: number, origin = new Date()): string {
  const date = new Date(origin);
  date.setDate(date.getDate() + offset);
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

export function overviewEventDay(event: EventItem): number {
  if (!event.startDate) return event.day;
  return (Date.parse(event.startDate) - Date.parse(localDate(0))) / 86_400_000;
}

export function useOverviewRange(
  accessToken: string,
  spaces: Space[],
  hidden: string[],
  filter: string,
  initialEvents: EventItem[],
  calendarReady: boolean,
  calendarRevision: number,
  preview?: { event?: EventItem },
) {
  const spaceKey = spaces
    .filter((space) => !hidden.includes(space.id))
    .map((space) => space.id)
    .sort()
    .join(",");
  const scope = `${calendarRevision}:${filter}:${spaceKey}`;
  const [range, setRange] = useState({
    scope,
    days: initialDays,
    events: [] as EventItem[],
  });
  const current =
    range.scope === scope ? range : { scope, days: initialDays, events: [] };
  const [request, setRequest] = useState<{
    scope: string;
    event: EventItem;
    days: number;
  }>();
  const [probe, setProbe] = useState<{
    key: string;
    event?: EventItem;
    error?: boolean;
  }>();
  const [failure, setFailure] = useState("");
  const [retry, setRetry] = useState(0);
  const key = `${scope}:${current.days}:${retry}`;
  const usePreview =
    current.days === initialDays &&
    hidden.length === 0 &&
    calendarRevision === 0 &&
    preview !== undefined;
  const loadedLater = initialEvents.some((event) => {
    const day = overviewEventDay(event);
    return day >= current.days && day <= 90;
  });

  useEffect(() => {
    setRequest((previous) =>
      previous?.scope === scope ? previous : undefined,
    );
  }, [scope]);

  useEffect(() => {
    if (
      !calendarReady ||
      filter === "tasks" ||
      !spaceKey ||
      loadedLater ||
      usePreview
    )
      return;
    const controller = new AbortController();
    void nextCalendarEvent(
      accessToken,
      localDate(current.days),
      spaceKey.split(","),
      controller.signal,
    )
      .then((event) => {
        if (!controller.signal.aborted) {
          const valid =
            event &&
            spaceKey.split(",").includes(event.space) &&
            overviewEventDay(event) >= current.days;
          setProbe({ key, event: valid ? event : undefined });
        }
      })
      .catch(() => {
        if (!controller.signal.aborted) setProbe({ key, error: true });
      });
    return () => controller.abort();
  }, [
    accessToken,
    calendarReady,
    current.days,
    filter,
    key,
    loadedLater,
    spaceKey,
    usePreview,
  ]);

  useEffect(() => {
    if (!request || request.scope !== scope) return;
    const controller = new AbortController();
    const from = request.event.startDate;
    if (!from) return;
    void (async () => {
      try {
        const items: EventItem[] = [];
        let offset: number | null = 0;
        while (offset !== null && !controller.signal.aborted) {
          const page = await listCalendarEvents(
            accessToken,
            offset,
            from,
            localDate(request.days - 1),
            controller.signal,
          );
          items.push(...page.items);
          offset = page.nextOffset;
        }
        if (controller.signal.aborted) return;
        const visibleSpaces = new Set(spaceKey.split(","));
        setRange((previous) => ({
          scope,
          days: request.days,
          events: [
            ...(previous.scope === scope ? previous.events : []),
            ...items.filter((item) => visibleSpaces.has(item.space)),
          ],
        }));
      } catch {
        if (!controller.signal.aborted) setFailure(key);
      } finally {
        if (!controller.signal.aborted) setRequest(undefined);
      }
    })();
    return () => controller.abort();
  }, [accessToken, key, request, scope, spaceKey]);

  const next = usePreview
    ? preview?.event
    : probe?.key === key
      ? probe.event
      : undefined;
  const loading = request?.scope === scope;
  const more = filter !== "tasks" && (loadedLater || Boolean(next?.startDate));
  return {
    days: current.days,
    events: current.events,
    loading,
    more,
    error: failure === key || (probe?.key === key && probe.error),
    retry: () => setRetry((value) => value + 1),
    expand: () => {
      if (loading) return;
      setFailure("");
      if (loadedLater) {
        setRange({ ...current, days: 91 });
      } else if (next?.startDate) {
        setRequest({
          scope,
          event: next,
          days: overviewEventDay(next) + initialDays,
        });
      }
    },
  };
}
