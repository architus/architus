//! The tables that are used in the postresql database to represent
//! the features that each guild has. This sets up a relational
//! database through two tables. The feature flags table represents
//! unique features that have an id, name, and bool for whether or
//! not it is an open feature. The guild geatures table simply
//! has a guild id and feature id for mapping a guild to what
//! features it has.

table! {
    tb_feature_flags {
        id -> Integer,
        name -> Text,
        open -> Bool,
    }
}

table! {
    // This table uses a composite key.
    tb_guild_features (guild_id, feature_id) {
        guild_id -> BigInt,
        feature_id -> Integer,
    }
}
