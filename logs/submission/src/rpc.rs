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
    }

    // Ignore clippy linting on generated code
    #[allow(clippy::all, clippy::pedantic, clippy::nursery)]
    pub mod revision {
        tonic::include_proto!("logs.revision");

        pub type Client = revision_service_client::RevisionServiceClient<tonic::transport::Channel>;
    }
}
