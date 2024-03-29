//! Contains generated code from the shared service Protobuf RPC definitions

pub mod feature_gate {
    mod generated {
        // Ignore clippy linting on generated code
        #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
        tonic::include_proto!("featuregate");
    }

    pub use generated::*;
}
