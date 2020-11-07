CREATE TABLE IF NOT EXISTS public.tb_twitch_subs (
    stream_user_id bigint NOT NULL,
    guild_id bigint NOT NULL,
    PRIMARY KEY (stream_user_id, guild_id)
);

GRANT ALL ON TABLE public.tb_twitch_subs TO autbot;
