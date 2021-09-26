CREATE TABLE IF NOT EXISTS channel_tracker {
    BIGINT channel_id,
    BIGINT start_date,
    BIGINT end_date,
    BOOLEAN backlog_done,
    PRIMARY_KEY(channel_id)
}
