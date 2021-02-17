CREATE TABLE IF NOT EXISTS public.tb_twitch_subs (
    stream_user_id bigint NOT NULL,
    guild_id bigint NOT NULL,
    PRIMARY KEY (stream_user_id, guild_id)
);

GRANT ALL ON TABLE public.tb_twitch_subs TO autbot;

CREATE TABLE IF NOT EXISTS public.tb_tokens (
    client_id varchar(64) NOT NULL,
    client_token varchar(64) NOT NULL,
    expires_at timestamp without time zone NOT NULL,
    PRIMARY KEY (client_id)
);

GRANT ALL ON TABLE public.tb_tokens TO autbot;