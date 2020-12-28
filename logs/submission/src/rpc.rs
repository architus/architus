//! Contains generated code from the shared service Protobuf RPC definitions

pub mod submission {
    mod generated {
        // Ignore clippy linting on generated code
        #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
        tonic::include_proto!("logs.submission");
    }

    pub use generated::*;
}

pub mod revision {
    mod generated {
        // Ignore clippy linting on generated code
        #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
        tonic::include_proto!("logs.revision");
    }

    pub use generated::*;
    pub type Client =
        generated::revision_service_client::RevisionServiceClient<tonic::transport::Channel>;
}
