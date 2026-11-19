UPDATE calendar_events
SET time_zone = 'Europe/Kyiv'
WHERE time_zone = 'Europe/Kiev';

UPDATE calendar_event_occurrence_overrides
SET time_zone = 'Europe/Kyiv'
WHERE time_zone = 'Europe/Kiev';

UPDATE users
SET time_zone = 'Europe/Kyiv'
WHERE time_zone = 'Europe/Kiev';

UPDATE auth_challenges
SET time_zone = 'Europe/Kyiv'
WHERE time_zone = 'Europe/Kiev';
