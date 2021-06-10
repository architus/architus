//! Contains generated code from the shared service Protobuf RPC definitions

// Ignore clippy linting on generated code
#[allow(clippy::all, clippy::pedantic, clippy::nursery)]
pub mod feature_gate {
    tonic::include_proto!("featuregate");

    pub type Client = feature_gate_client::FeatureGateClient<tonic::transport::Channel>;
}

pub mod logs {
    // Ignore clippy linting on generated code
    #[allow(clippy::all, clippy::pedantic, clippy::nursery)]
    pub mod uptime {
        tonic::include_proto!("logs.uptime");

        pub type Client = uptime_service_client::UptimeServiceClient<tonic::transport::Channel>;
    }
}

/// Transforms an RPC result into a more useful one,
/// and prepares a backoff error for potentially recoverable tonic Status's
pub fn into_backoff<T>(
    result: Result<tonic::Response<T>, tonic::Status>,
) -> Result<T, backoff::Error<tonic::Status>> {
    match result {
        Ok(response) => Ok(response.into_inner()),
        Err(status) => match status.code() {
            tonic::Code::Internal | tonic::Code::Unknown | tonic::Code::Unavailable => {
                Err(backoff::Error::Permanent(status))
            }
            _ => Err(backoff::Error::Transient(status)),
        },
    }
}
