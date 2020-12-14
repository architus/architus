//! Contains generated code from the shared service Protobuf RPC definitions

pub mod feature_gate {
    // Ignore clippy linting on generated code
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    tonic::include_proto!("featuregate");
}

pub type FeatureGateClient =
    feature_gate::feature_gate_client::FeatureGateClient<tonic::transport::Channel>;

pub mod uptime {
    // Ignore clippy linting on generated code
    #![allow(clippy::all, clippy::pedantic, clippy::nursery)]
    tonic::include_proto!("logs.uptime");
}

pub type LogsUptimeClient =
    uptime::uptime_service_client::UptimeServiceClient<tonic::transport::Channel>;
