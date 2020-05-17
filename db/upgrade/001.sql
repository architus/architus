
CREATE TABLE public.tb_react_events (
    message_id bigint NOT NULL,
    guild_id bigint NOT NULL,
    channel_id bigint NOT NULL,
    command smallint NOT NULL,
    expires_on timestamp without time zone NOT NULL,
    created_on timestamp without time zone NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id, message_id)
);

GRANT ALL ON TABLE public.tb_react_events TO autbot;
