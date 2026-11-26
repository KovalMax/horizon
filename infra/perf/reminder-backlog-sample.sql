SELECT floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint,
       count(*) FILTER (WHERE status = 'pending'),
       count(*) FILTER (WHERE status = 'sending'),
       count(*) FILTER (WHERE status = 'sent'),
       count(*) FILTER (WHERE status IN ('failed','cancelled')),
       count(*) FILTER (WHERE attempts > 1),
       coalesce(round(extract(epoch FROM clock_timestamp() - min(scheduled_at)
           FILTER (WHERE status = 'pending')) * 1000),0)::bigint,
       (SELECT count(*) FROM calendar_reminder_deliveries
         WHERE user_id = '00000000-0000-0000-0000-00000000a001'),
       (SELECT count(*) FROM pg_stat_activity
         WHERE datname = current_database() AND pid <> pg_backend_pid() AND wait_event_type = 'Lock')
FROM task_reminder_jobs WHERE recipient_id = '00000000-0000-0000-0000-00000000a001'
