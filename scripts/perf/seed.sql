\set ON_ERROR_STOP on
SET TIME ZONE 'UTC';
CREATE EXTENSION pg_stat_statements;

-- Stable UUIDs, row counts, and membership distribution. Dates are relative so
-- the real browser's current Calendar window contains the same mix each run.
CREATE FUNCTION pg_temp.fixture_id(text) RETURNS uuid LANGUAGE sql IMMUTABLE
RETURN md5('horizon-perf:' || $1)::uuid;

CREATE TEMP TABLE profiles AS
SELECT kind, spaces, peers, lists, tasks, places, events, notifications,
       pg_temp.fixture_id(kind) AS user_id
FROM (VALUES
  ('typical', 4, 5, 4, 25, 60, 20, 400),
  ('large', 24, 100, 8, 65, 240, 120, 15000)
) p(kind, spaces, peers, lists, tasks, places, events, notifications);

INSERT INTO users (id)
SELECT user_id FROM profiles
UNION ALL
SELECT pg_temp.fixture_id(kind || ':peer:' || n)
FROM profiles CROSS JOIN LATERAL generate_series(1, peers) n;
INSERT INTO email_identities (id, user_id, email_normalized, verified_at)
SELECT pg_temp.fixture_id(kind || ':email'), user_id,
       'perf-' || kind || '@example.test', now()
FROM profiles
UNION ALL
SELECT pg_temp.fixture_id(kind || ':peer-email:' || n),
       pg_temp.fixture_id(kind || ':peer:' || n),
       'perf-' || kind || '-peer-' || n || '@example.test', now()
FROM profiles CROSS JOIN LATERAL generate_series(1, peers) n;

CREATE TEMP TABLE fixture_spaces AS
SELECT p.*, n AS space_no, pg_temp.fixture_id(kind || ':space:' || n) AS space_id
FROM profiles p CROSS JOIN LATERAL generate_series(1, spaces) n;
INSERT INTO spaces (id, name, kind, created_at)
SELECT space_id, 'Perf ' || kind || ' ' || space_no, 'shared',
       now() + space_no * interval '1 second'
FROM fixture_spaces;
INSERT INTO space_memberships (space_id, user_id, role)
SELECT space_id, user_id, CASE WHEN space_no <= 8 THEN 'owner' ELSE 'editor' END
FROM fixture_spaces
UNION ALL
SELECT space_id, pg_temp.fixture_id(kind || ':peer:1'), 'owner'
FROM fixture_spaces WHERE space_no > 8
UNION ALL
SELECT space_id, pg_temp.fixture_id(kind || ':peer:' || n), 'viewer'
FROM fixture_spaces CROSS JOIN LATERAL generate_series(1, peers) n WHERE space_no = 1;

-- Unrelated data makes membership filters selective even for the large account.
INSERT INTO spaces (id, name, kind)
SELECT pg_temp.fixture_id('unrelated:' || n), 'Unrelated ' || n, 'shared'
FROM generate_series(1, 32) n;
INSERT INTO space_memberships (space_id, user_id, role)
SELECT pg_temp.fixture_id('unrelated:' || n), pg_temp.fixture_id('large:peer:2'),
       'owner'
FROM generate_series(1, 32) n;

CREATE TEMP TABLE fixture_lists AS
SELECT s.*, n AS list_no,
       pg_temp.fixture_id(space_id::text || ':list:' || n) AS list_id
FROM fixture_spaces s CROSS JOIN LATERAL generate_series(1, lists) n;
INSERT INTO todo_lists (id, space_id, name, created_at)
SELECT list_id, space_id, 'List ' || list_no, now() - list_no * interval '1 day'
FROM fixture_lists;
INSERT INTO tasks (id, list_id, creator_id, title, due_date, due_time, done,
                   assignee_id, reminder_offsets_minutes, created_at)
SELECT pg_temp.fixture_id(list_id::text || ':task:' || n), list_id, user_id,
       'Task ' || n, CASE WHEN n % 5 <> 0 THEN current_date + 2 + n % 27 END,
       CASE WHEN n % 4 = 0 THEN time '14:00' END, n % 3 = 0,
       CASE WHEN n % 2 = 0 THEN user_id END,
       CASE WHEN n % 2 = 0 THEN ARRAY[60, 1440] ELSE ARRAY[]::integer[] END,
       now() - n * interval '1 hour'
FROM fixture_lists CROSS JOIN LATERAL generate_series(1, tasks) n;
INSERT INTO saved_places (id, space_id, creator_id, name, category, address)
SELECT pg_temp.fixture_id(space_id::text || ':place:' || n), space_id, user_id,
       'Place ' || lpad(n::text, 4, '0'),
       (ARRAY['Food & drink', 'Groceries', 'Health', 'Services'])[n % 4 + 1],
       '12 Fixture Street'
FROM fixture_spaces CROSS JOIN LATERAL generate_series(1, places) n;

INSERT INTO calendar_events (id, space_id, creator_id, title, starts_at, ends_at,
                             time_zone, recurrence, recurrence_ends_on)
SELECT pg_temp.fixture_id(space_id::text || ':event:' || n), space_id, user_id,
       'Event ' || n, (d + time '14:00') AT TIME ZONE zone,
       (d + time '15:00') AT TIME ZONE zone, zone,
       CASE WHEN n % 3 = 0 THEN 'daily' WHEN n % 3 = 1 THEN 'weekly' END,
       CASE WHEN n % 6 = 0 THEN current_date + 45 END
FROM fixture_spaces CROSS JOIN LATERAL generate_series(1, events) n
CROSS JOIN LATERAL (
  SELECT current_date + (CASE WHEN n % 3 <> 2 THEN - (n % 120) ELSE n % 85 END) AS d,
         (ARRAY['UTC', 'Europe/Berlin', 'America/New_York'])[n % 3 + 1] AS zone
) start;
INSERT INTO calendar_event_occurrence_overrides (event_id, occurrence_date, cancelled)
SELECT e.id, current_date + n, true
FROM calendar_events e CROSS JOIN generate_series(1, 7) n
WHERE recurrence = 'daily' AND get_byte(uuid_send(e.id), 15) % 4 = 0;
INSERT INTO calendar_event_occurrence_overrides
    (event_id, occurrence_date, title, starts_at, ends_at, time_zone, all_day)
SELECT e.id, current_date + 10, 'Moved occurrence',
       (current_date + 2 + time '13:00') AT TIME ZONE e.time_zone,
       (current_date + 2 + time '14:00') AT TIME ZONE e.time_zone, e.time_zone, false
FROM calendar_events e
WHERE recurrence = 'daily' AND get_byte(uuid_send(e.id), 15) % 4 = 1;

INSERT INTO todo_lists (id, space_id, name)
SELECT pg_temp.fixture_id(s.id::text || ':background-list'), s.id, 'Background list'
FROM spaces s WHERE s.name LIKE 'Unrelated %';
INSERT INTO tasks (id, list_id, creator_id, title, due_date, done)
SELECT pg_temp.fixture_id(l.id::text || ':background-task:' || n), l.id,
       pg_temp.fixture_id('large:peer:2'), 'Background Task ' || n,
       current_date + n % 60, true
FROM todo_lists l CROSS JOIN generate_series(1, 150) n
WHERE l.name = 'Background list';
INSERT INTO saved_places (id, space_id, creator_id, name, category, address)
SELECT pg_temp.fixture_id(s.id::text || ':background-place:' || n), s.id,
       pg_temp.fixture_id('large:peer:2'), 'Background Place ' || n, 'Food & drink',
       '12 Fixture Street'
FROM spaces s CROSS JOIN generate_series(1, 180) n WHERE s.name LIKE 'Unrelated %';
INSERT INTO calendar_events (id, space_id, creator_id, title, starts_at, time_zone)
SELECT pg_temp.fixture_id(s.id::text || ':background-event:' || n), s.id,
       pg_temp.fixture_id('large:peer:2'), 'Background event ' || n,
       (current_date + n + time '16:00') AT TIME ZONE 'UTC', 'UTC'
FROM spaces s CROSS JOIN generate_series(1, 80) n WHERE s.name LIKE 'Unrelated %';

INSERT INTO notifications (id, recipient_id, space_id, event_type, title, body,
                           destination, created_at, read_at)
SELECT pg_temp.fixture_id(kind || ':notification:' || n), user_id, space_id,
       'event_change', 'Fixture Notification', 'A Calendar event changed.', 'calendar',
       now() - n * interval '1 minute', CASE WHEN n % 4 <> 0 THEN now() END
FROM fixture_spaces s CROSS JOIN LATERAL generate_series(1, notifications) n
WHERE space_no = 1;
ANALYZE;

SELECT kind, spaces, peers AS fanout_recipients, spaces * lists AS lists,
       spaces * lists * tasks AS tasks, spaces * places AS places,
       spaces * events AS event_series_and_singles, notifications
FROM profiles ORDER BY spaces;
SELECT count(*) AS occurrence_overrides FROM calendar_event_occurrence_overrides;
SELECT count(*) AS unrelated_spaces FROM spaces WHERE name LIKE 'Unrelated %';
