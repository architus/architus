//! Contains generated code from the shared service Protobuf RPC definitions

pub mod logs {
    // Ignore clippy linting on generated code
    #[allow(clippy::all, clippy::pedantic, clippy::nursery)]
    pub mod event {
        tonic::include_proto!("logs.event");
    }

    // Ignore clippy linting on generated code
    #[allow(clippy::all, clippy::pedantic, clippy::nursery)]
    pub mod submission {
        tonic::include_proto!("logs.submission");

        pub type Client =
            submission_service_client::SubmissionServiceClient<tonic::transport::Channel>;
    }
}

// Ignore clippy linting on generated code
#[allow(clippy::all, clippy::pedantic, clippy::nursery)]
pub mod gateway_queue_lib {
    tonic::include_proto!("gateway_queue_lib");
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
