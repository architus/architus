--
-- Name: tb_user_connections; Type: TABLE; Schema: public; Owner: autbot
--

CREATE TABLE IF NOT EXISTS public.tb_user_connections (
    id BIGINT PRIMARY KEY,
    user_id BIGINT,
    account_type TEXT,
    username TEXT,
    visibility INT,
    show_activity BOOLEAN
);


ALTER TABLE public.tb_user_connections OWNER TO autbot;