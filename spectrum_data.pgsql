--
-- PostgreSQL database dump
--

-- Dumped from database version 9.5.13
-- Dumped by pg_dump version 9.5.13

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
-- Name: tb_users; Type: TABLE; Schema: public; Owner: autbot
--

CREATE TABLE public.tb_users (
    user_id integer NOT NULL,
    discord_id character varying(50),
    aut_score integer,
    norm_score integer,
    nice_score integer,
    toxic_score integer
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
-- Data for Name: tb_users; Type: TABLE DATA; Schema: public; Owner: autbot
--

COPY public.tb_users (user_id, discord_id, aut_score, norm_score, nice_score, toxic_score) FROM stdin;
2	218157264333307905	2	2	2	2
4	186304444378382336	2	2	2	2
5	124733533661954050	2	4	2	2
12	131776062739841024	2	2	2	2
3	214037134477230080	4	2	2	3
13	179636744818262016	2	2	2	2
7	189528269547110400	2	2	2	2
10	250802771068977153	3	2	2	4
14	131857650676793345	2	2	2	2
8	257260785258987530	2	2	2	2
9	130959008587710464	3	2	2	2
1	168722115447488512	12	7	2	4
6	178700066091958273	3	2	7	6
15	97544912039342080	2	2	2	2
11	245298512755949569	2	2	2	5
\.


--
-- Name: tb_users_user_id_seq; Type: SEQUENCE SET; Schema: public; Owner: autbot
--

SELECT pg_catalog.setval('public.tb_users_user_id_seq', 15, true);


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
-- PostgreSQL database dump complete
--

