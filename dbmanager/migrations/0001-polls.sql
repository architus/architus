CREATE TABLE IF NOT EXISTS public.tb_react_events (
    message_id bigint NOT NULL,
    guild_id bigint NOT NULL,
    channel_id bigint NOT NULL,
    event_type smallint NOT NULL,
    payload text NOT NULL,
    expires_on timestamp without time zone NOT NULL,
    created_on timestamp without time zone NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, message_id)
);

GRANT ALL ON TABLE public.tb_react_events TO autbot;
