\set ON_ERROR_STOP on
SET TIME ZONE 'UTC';
INSERT INTO users (id) VALUES ('00000000-0000-0000-0000-00000000a001');
INSERT INTO email_identities (id,user_id,email_normalized,verified_at)
VALUES (gen_random_uuid(),'00000000-0000-0000-0000-00000000a001','perf-reminders@example.test',now());
INSERT INTO spaces (id,name,kind)
VALUES ('00000000-0000-0000-0000-00000000a002','Reminder backlog fixture','shared');
INSERT INTO space_memberships VALUES
('00000000-0000-0000-0000-00000000a002','00000000-0000-0000-0000-00000000a001','owner',now());
INSERT INTO todo_lists (id,space_id,name)
VALUES ('00000000-0000-0000-0000-00000000a003','00000000-0000-0000-0000-00000000a002','Backlog Tasks');
INSERT INTO notification_preferences (user_id,event_type,channel,enabled)
VALUES ('00000000-0000-0000-0000-00000000a001','due_reminder','email',true);
WITH fixture AS (SELECT now() + interval '55 minutes' AS due_at), inserted AS (
  INSERT INTO tasks (id,list_id,creator_id,title,due_date,due_time,reminder_offsets_minutes)
  SELECT gen_random_uuid(),'00000000-0000-0000-0000-00000000a003',
    '00000000-0000-0000-0000-00000000a001', 'Backlog Task ' || n, due_at::date,due_at::time,ARRAY[60]
  FROM fixture CROSS JOIN generate_series(1,600) n
  RETURNING id,creator_id
)
INSERT INTO task_reminder_jobs (id,task_id,recipient_id,offset_minutes,scheduled_at,available_at)
SELECT gen_random_uuid(),id,creator_id,60,due_at - interval '1 hour',due_at - interval '1 hour'
FROM inserted CROSS JOIN fixture;

WITH inserted AS (
  INSERT INTO calendar_events (id,space_id,creator_id,title,starts_at,time_zone)
  SELECT gen_random_uuid(),'00000000-0000-0000-0000-00000000a002',
    '00000000-0000-0000-0000-00000000a001','Backlog event ' || n,now() + interval '59 minutes','UTC'
  FROM generate_series(1,130) n
  RETURNING id,creator_id
)
INSERT INTO calendar_reminder_preferences (event_id,user_id,offsets_minutes)
SELECT id,creator_id,ARRAY[60] FROM inserted;
ANALYZE;
