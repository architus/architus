use crate::event::Content;
use crate::gateway::path::Path;
use crate::gateway::source::{AuditLogSource, OnFailure, Source};
use crate::gateway::{Context, Processor, ProcessorFleet};
use crate::rpc::submission::EventType;
use jmespath::Variable;
use twilight_model::gateway::event::EventType as GatewayEventType;

/// Registers all pre-configured processors
/// to handle as many gateway events as possible
pub fn register_all(fleet: ProcessorFleet) -> ProcessorFleet {
    fleet.register(
        GatewayEventType::ChannelCreate,
        // TODO implement
        Processor {
            audit_log: Some(AuditLogSource::new(
                |_ctx| async move { Err(anyhow::anyhow!("not implemented")) },
                OnFailure::Abort,
            )),
            event_type: Source::Constant(EventType::ChannelCreate),
            timestamp: Source::gateway(
                Path::from("id"),
                |var, ctx| extract_string_id(var, ctx).map(|id| architus_id::extract_timestamp(id)),
                OnFailure::Abort,
            ),
            reason: Source::audit_log(
                Path::from("reason"),
                extract_string_option,
                OnFailure::Or(None),
            ),
            channel: Source::Constant(None),
            agent: Source::Constant(None),
            subject: Source::Constant(None),
            auxiliary: Source::Constant(None),
            content: Source::sync_fn(|_| Ok(Content::make("")), OnFailure::Abort),
        },
    )
}

/// Extractor function for a `Option<String>`
fn extract_string_option(
    variable: &Variable,
    _ctx: Context<'_>,
) -> Result<Option<String>, anyhow::Error> {
    match variable {
        Variable::String(s) => Ok(Some(s.clone())),
        _ => Ok(None),
    }
}

/// Extractor function for a `u64` from a JSON string
fn extract_string_id(variable: &Variable, _ctx: Context<'_>) -> Result<u64, anyhow::Error> {
    match variable {
        Variable::String(s) => u64::parse(s).context("cannot extract u64 from JSON string"),
        _ => Err(anyhow::anyhow!(
            "variable was not of type String: {:?}",
            variable
        )),
    }
}
