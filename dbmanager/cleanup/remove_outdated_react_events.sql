DELETE FROM tb_react_events
WHERE message_id NOT IN (
    SELECT TOP 1000 message_id from tb_react_events ORDER BY created_on desc
);
