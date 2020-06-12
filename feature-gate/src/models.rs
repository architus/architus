use crate::schema::{tb_feature_flags, tb_guild_features};

/// Represents a feature that the architus bot offers to
/// users / guilds.
#[derive(Queryable)]
pub struct Feature {
    pub id: i32,
    pub name: String,
    pub open: bool,
}

/// A guild <-> feature association in the database.
#[derive(Queryable, Insertable)]
#[table_name = "tb_guild_features"]
pub struct Guild {
    pub guild_id: i64,
    pub feature_id: i32,
}

/// A struct for inserting a new feature into the database.
#[derive(Insertable)]
#[table_name = "tb_feature_flags"]
pub struct NewFeature<'feature> {
    pub name: &'feature str,
    pub open: bool,
}
