#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Void {
    #[prost(bool, tag = "1")]
    pub val: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ShardInfo {
    #[prost(uint32, tag = "1")]
    pub shard_id: u32,
    #[prost(uint32, tag = "2")]
    pub shard_count: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GuildInfo {
    #[prost(uint32, tag = "1")]
    pub guild_count: u32,
    #[prost(uint32, tag = "2")]
    pub user_count: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ShardId {
    #[prost(uint32, tag = "1")]
    pub shard_id: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct File {
    #[prost(string, tag = "1")]
    pub location: std::string::String,
    #[prost(string, tag = "2")]
    pub name: std::string::String,
    #[prost(string, tag = "3")]
    pub filetype: std::string::String,
    #[prost(bytes, tag = "4")]
    pub file: std::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Url {
    #[prost(string, tag = "1")]
    pub url: std::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Guild {
    #[prost(fixed64, tag = "1")]
    pub id: u64,
    #[prost(string, tag = "2")]
    pub name: std::string::String,
    #[prost(string, tag = "3")]
    pub icon: std::string::String,
    #[prost(string, tag = "4")]
    pub splash: std::string::String,
    #[prost(fixed64, tag = "5")]
    pub owner_id: u64,
    #[prost(string, tag = "6")]
    pub region: std::string::String,
    #[prost(uint32, tag = "7")]
    pub afk_timeout: u32,
    #[prost(bool, tag = "8")]
    pub unavailable: bool,
    #[prost(int32, tag = "9")]
    pub max_members: i32,
    #[prost(string, tag = "10")]
    pub banner: std::string::String,
    #[prost(string, tag = "11")]
    pub description: std::string::String,
    #[prost(int32, tag = "12")]
    pub mfa_level: i32,
    #[prost(uint32, tag = "13")]
    pub premium_tier: u32,
    #[prost(int32, tag = "14")]
    pub premium_subscription_count: i32,
    #[prost(string, tag = "15")]
    pub preferred_locale: std::string::String,
    #[prost(int32, tag = "16")]
    pub member_count: i32,
    #[prost(string, repeated, tag = "17")]
    pub features: ::std::vec::Vec<std::string::String>,
}
#[doc = r" Generated client implementations."]
pub mod manager_client {
    #![allow(unused_variables, dead_code, missing_docs)]
    use tonic::codegen::*;
    pub struct ManagerClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl ManagerClient<tonic::transport::Channel> {
        #[doc = r" Attempt to create a new client by connecting to a given endpoint."]
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> ManagerClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::ResponseBody: Body + HttpBody + Send + 'static,
        T::Error: Into<StdError>,
        <T::ResponseBody as HttpBody>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_interceptor(inner: T, interceptor: impl Into<tonic::Interceptor>) -> Self {
            let inner = tonic::client::Grpc::with_interceptor(inner, interceptor);
            Self { inner }
        }
        pub async fn register(
            &mut self,
            request: impl tonic::IntoRequest<super::Void>,
        ) -> Result<tonic::Response<super::ShardInfo>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/manager.Manager/register");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn guild_count(
            &mut self,
            request: impl tonic::IntoRequest<super::Void>,
        ) -> Result<tonic::Response<super::GuildInfo>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/manager.Manager/guild_count");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn checkin(
            &mut self,
            request: impl tonic::IntoRequest<super::ShardId>,
        ) -> Result<tonic::Response<super::Void>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/manager.Manager/checkin");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn publish_file(
            &mut self,
            request: impl tonic::IntoRequest<super::File>,
        ) -> Result<tonic::Response<super::Url>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/manager.Manager/publish_file");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn all_guilds(
            &mut self,
            request: impl tonic::IntoRequest<super::Void>,
        ) -> Result<tonic::Response<tonic::codec::Streaming<super::Guild>>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/manager.Manager/all_guilds");
            self.inner
                .server_streaming(request.into_request(), path, codec)
                .await
        }
        pub async fn guild_update(
            &mut self,
            request: impl tonic::IntoStreamingRequest<Message = super::Guild>,
        ) -> Result<tonic::Response<super::Void>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/manager.Manager/guild_update");
            self.inner
                .client_streaming(request.into_streaming_request(), path, codec)
                .await
        }
    }
    impl<T: Clone> Clone for ManagerClient<T> {
        fn clone(&self) -> Self {
            Self {
                inner: self.inner.clone(),
            }
        }
    }
    impl<T> std::fmt::Debug for ManagerClient<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "ManagerClient {{ ... }}")
        }
    }
}
