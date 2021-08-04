CREATE TABLE IF NOT EXISTS public.tb_roles (
    guild_id BIGINT NOT NULL,
    message_id BIGINT NOT NULL,
    role_id BIGINT NOT NULL,
    emoji TEXT NOT NULL,
    PRIMARY KEY (role_id)
);

GRANT ALL ON TABLE public.tb_roles TO autbot;