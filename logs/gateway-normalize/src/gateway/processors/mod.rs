// It's common to pass Context<'_> by value, so ignore the linter
#![allow(clippy::needless_pass_by_value)]

mod interaction;
mod member;
mod message;
mod reaction;

use crate::gateway::{ProcessorContext, ProcessorFleet};
use anyhow::Context as _;
use jmespath::Variable;
use serde::de::{DeserializeOwned, DeserializeSeed};
use twilight_model::guild::member::MemberDeserializer;
use twilight_model::guild::Member;
use twilight_model::id::GuildId;

/// Registers all pre-configured processors
/// to handle all gateway-originating events
pub fn register_all(fleet: &mut ProcessorFleet) {
    member::register_all(fleet);
    message::register_all(fleet);
    reaction::register_all(fleet);
    interaction::register_all(fleet);
}

/// u64 extractor that returns the underlying timestamp for a snowflake-encoded ID
#[allow(clippy::unnecessary_wraps)]
#[allow(dead_code)]
const fn timestamp_from_id(id: u64, _ctx: &ProcessorContext<'_>) -> Result<u64, anyhow::Error> {
    Ok(architus_id::snowflake::extract_timestamp(id))
}

/// Extractor function for a `u64` from a JSON string
fn extract_id(variable: &Variable, _ctx: &ProcessorContext<'_>) -> Result<u64, anyhow::Error> {
    match variable {
        Variable::String(s) => s
            .parse::<u64>()
            .context("cannot extract u64 from JSON string"),
        _ => Err(anyhow::anyhow!(
            "variable was not of type String: {:?}",
            variable
        )),
    }
}

/// Performs a generic extraction to create a value T from JSON
fn extract<T>(variable: &Variable, _ctx: &ProcessorContext<'_>) -> Result<T, anyhow::Error>
where
    T: DeserializeOwned,
{
    let value = serde_json::to_value(variable)?;
    let t = serde_json::from_value::<T>(value)?;
    Ok(t)
}

/// Attempts to extract a member struct using twilight's `MemberDeserializer` struct
/// and the guild id associated with the context's inner event struct
fn extract_member(
    variable: &Variable,
    ctx: &ProcessorContext<'_>,
) -> Result<Member, anyhow::Error> {
    let member_deserializer = MemberDeserializer::new(GuildId(ctx.event.guild_id));
    let value = serde_json::to_value(variable)?;
    let member = member_deserializer.deserialize(value).with_context(|| {
        format!(
            "could not deserialize Member value for guild id {}",
            ctx.event.guild_id
        )
    })?;
    Ok(member)
}
