use anyhow::{Context, Result};
use serde::Serialize;

/// Attempts to serialize a simple value to retrieve its literal representation
/// without leading or trailing quotes
pub fn value_to_string<E>(val: &E) -> Result<String>
where
    E: Serialize,
{
    let pattern: &[_] = &['\'', '"'];
    let event_type_str = serde_json::to_string(val)
        .with_context(|| format!("{:?} was not serializable", std::any::type_name::<E>()))?;
    Ok(String::from(event_type_str.trim_matches(pattern)))
}
