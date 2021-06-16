--
-- Name: tb_usage_analytics; Type: TABLE; Schema: public; Owner: autbot
--

CREATE TABLE IF NOT EXISTS public.tb_usage_analytics (
    id SERIAL PRIMARY KEY,
    prefix TEXT,
    command TEXT,
    guild_id BIGINT,
    channel_id BIGINT,
    author_id BIGINT
);


ALTER TABLE public.tb_usage_analytics OWNER TO autbot;