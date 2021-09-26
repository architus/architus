CREATE TABLE IF NOT EXISTS scrape_tracker {
    BIGINT guild_id,
    BIGINT start_date,
    BIGINT end_date,
    BOOLEAN backlog_done,
    PRIMARY_KEY(guild_id)
}
