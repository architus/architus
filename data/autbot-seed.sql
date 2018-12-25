--
-- PostgreSQL database dump
--

-- Dumped from database version 9.5.14
-- Dumped by pg_dump version 9.5.14

SET statement_timeout = 0;
SET lock_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: plpgsql; Type: EXTENSION; Schema: -; Owner: 
--

CREATE EXTENSION IF NOT EXISTS plpgsql WITH SCHEMA pg_catalog;


--
-- Name: EXTENSION plpgsql; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION plpgsql IS 'PL/pgSQL procedural language';


SET default_tablespace = '';

SET default_with_oids = false;

--
-- Name: tb_admins; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.tb_admins (
    discord_id bigint NOT NULL,
    server_id bigint NOT NULL,
    username text NOT NULL
);


ALTER TABLE public.tb_admins OWNER TO postgres;

--
-- Name: tb_commands; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.tb_commands (
    trigger text NOT NULL,
    response text NOT NULL,
    count bigint NOT NULL,
    server_id bigint NOT NULL,
    author_id bigint
);


ALTER TABLE public.tb_commands OWNER TO postgres;

--
-- Name: tb_roles; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.tb_roles (
    target_role_id bigint NOT NULL,
    server_id bigint NOT NULL,
    required_role_id bigint
);


ALTER TABLE public.tb_roles OWNER TO postgres;

--
-- Name: tb_settings; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.tb_settings (
    server_id bigint NOT NULL,
    json_blob text NOT NULL
);


ALTER TABLE public.tb_settings OWNER TO postgres;

--
-- Name: tb_users; Type: TABLE; Schema: public; Owner: autbot
--

CREATE TABLE public.tb_users (
    user_id integer NOT NULL,
    discord_id character varying(50),
    aut_score integer,
    norm_score integer,
    nice_score integer,
    toxic_score integer,
    awareness_score integer
);


ALTER TABLE public.tb_users OWNER TO autbot;

--
-- Name: tb_users_user_id_seq; Type: SEQUENCE; Schema: public; Owner: autbot
--

CREATE SEQUENCE public.tb_users_user_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER TABLE public.tb_users_user_id_seq OWNER TO autbot;

--
-- Name: tb_users_user_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: autbot
--

ALTER SEQUENCE public.tb_users_user_id_seq OWNED BY public.tb_users.user_id;


--
-- Name: user_id; Type: DEFAULT; Schema: public; Owner: autbot
--

ALTER TABLE ONLY public.tb_users ALTER COLUMN user_id SET DEFAULT nextval('public.tb_users_user_id_seq'::regclass);


--
-- Name: tb_users_user_id_seq; Type: SEQUENCE SET; Schema: public; Owner: autbot
--

SELECT pg_catalog.setval('public.tb_users_user_id_seq', 98, true);


--
-- Name: tb_admins_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tb_admins
    ADD CONSTRAINT tb_admins_pkey PRIMARY KEY (discord_id);


--
-- Name: tb_commands_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tb_commands
    ADD CONSTRAINT tb_commands_pkey PRIMARY KEY (trigger);


--
-- Name: tb_roles_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tb_roles
    ADD CONSTRAINT tb_roles_pkey PRIMARY KEY (target_role_id);


--
-- Name: tb_settings_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.tb_settings
    ADD CONSTRAINT tb_settings_pkey PRIMARY KEY (server_id);


--
-- Name: tb_users_pkey; Type: CONSTRAINT; Schema: public; Owner: autbot
--

ALTER TABLE ONLY public.tb_users
    ADD CONSTRAINT tb_users_pkey PRIMARY KEY (user_id);


--
-- Name: SCHEMA public; Type: ACL; Schema: -; Owner: postgres
--

REVOKE ALL ON SCHEMA public FROM PUBLIC;
REVOKE ALL ON SCHEMA public FROM postgres;
GRANT ALL ON SCHEMA public TO postgres;
GRANT ALL ON SCHEMA public TO PUBLIC;


--
-- Name: TABLE tb_admins; Type: ACL; Schema: public; Owner: postgres
--

REVOKE ALL ON TABLE public.tb_admins FROM PUBLIC;
REVOKE ALL ON TABLE public.tb_admins FROM postgres;
GRANT ALL ON TABLE public.tb_admins TO postgres;
GRANT ALL ON TABLE public.tb_admins TO autbot;


--
-- Name: TABLE tb_commands; Type: ACL; Schema: public; Owner: postgres
--

REVOKE ALL ON TABLE public.tb_commands FROM PUBLIC;
REVOKE ALL ON TABLE public.tb_commands FROM postgres;
GRANT ALL ON TABLE public.tb_commands TO postgres;
GRANT ALL ON TABLE public.tb_commands TO autbot;


--
-- Name: TABLE tb_roles; Type: ACL; Schema: public; Owner: postgres
--

REVOKE ALL ON TABLE public.tb_roles FROM PUBLIC;
REVOKE ALL ON TABLE public.tb_roles FROM postgres;
GRANT ALL ON TABLE public.tb_roles TO postgres;
GRANT ALL ON TABLE public.tb_roles TO autbot;


--
-- Name: TABLE tb_settings; Type: ACL; Schema: public; Owner: postgres
--

REVOKE ALL ON TABLE public.tb_settings FROM PUBLIC;
REVOKE ALL ON TABLE public.tb_settings FROM postgres;
GRANT ALL ON TABLE public.tb_settings TO postgres;
GRANT ALL ON TABLE public.tb_settings TO autbot;


--
-- Name: TABLE tb_users; Type: ACL; Schema: public; Owner: autbot
--

REVOKE ALL ON TABLE public.tb_users FROM PUBLIC;
REVOKE ALL ON TABLE public.tb_users FROM autbot;
GRANT ALL ON TABLE public.tb_users TO autbot;


--
-- PostgreSQL database dump complete
--

