--
-- Name: tb_feature_flags; Type: TABLE; Schema: public; Owner: autbot
--

CREATE TABLE public.tb_feature_flags (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    open BOOL NOT NULL DEFAULT false
);


ALTER TABLE public.tb_feature_flags OWNER TO autbot;

--
-- Name: tb_guild_features; Type: TABLE; Schema: public; Owner: autbot
--

CREATE TABLE public.tb_guild_features (
    guild_id bigint NOT NULL,
    feature_id INTEGER NOT NULL,

    PRIMARY KEY(guild_id, feature_id),
    FOREIGN KEY(feature_id) REFERENCES public.tb_feature_flags(id)
);


ALTER TABLE public.tb_guild_features OWNER TO autbot;
