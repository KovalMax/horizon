SELECT jsonb_pretty(jsonb_build_object(
  'postgres_version', current_setting('server_version'),
  'shared_buffers', current_setting('shared_buffers'),
  'work_mem', current_setting('work_mem'),
  'sample_date_utc', :'date',
  'reminder_jobs', (SELECT count(*) FROM task_reminder_jobs),
  'reminder_jobs_created_during_mixed', (
    SELECT count(*) FROM task_reminder_jobs
    WHERE created_at >= NULLIF(:'mixed_start', '')::timestamptz
      AND created_at <= NULLIF(:'mixed_end', '')::timestamptz
  ),
  'statements', (SELECT jsonb_agg(to_jsonb(top_queries)) FROM (
    SELECT calls, rows, round(total_exec_time::numeric, 2) AS total_ms,
           round(mean_exec_time::numeric, 3) AS mean_ms,
           shared_blks_hit, shared_blks_read, temp_blks_read, temp_blks_written,
           round(shared_blk_read_time::numeric, 3) AS physical_read_ms, query
    FROM pg_stat_statements
    WHERE dbid = (SELECT oid FROM pg_database WHERE datname = current_database())
    ORDER BY total_exec_time DESC
  ) top_queries)
));
