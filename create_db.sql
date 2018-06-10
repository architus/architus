CREATE TABLE IF NOT EXISTS tb_users(
  user_id           SERIAL PRIMARY KEY,
  discord_id        VARCHAR(50),
  aut_score         INT,
  norm_score        INT,
  nice_score        INT,
  toxic_score       INT
);
