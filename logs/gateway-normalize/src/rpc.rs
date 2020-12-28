//! Contains generated code from the shared service Protobuf RPC definitions

pub mod submission {
    // Ignore clippy linting on generated code
    mod generated {
        #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
        tonic::include_proto!("logs.submission");
    }

    pub use generated::*;
    pub type Client =
        generated::submission_service_client::SubmissionServiceClient<tonic::transport::Channel>;
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
