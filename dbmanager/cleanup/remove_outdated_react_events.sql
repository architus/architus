DELETE FROM tb_react_events
WHERE NOW() > expires_on;
