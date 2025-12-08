//! # xds-types
//!
//! Generated protobuf types for Envoy xDS APIs.
//!
//! This crate provides Rust types generated from the Envoy xDS protobuf
//! definitions. It includes:
//!
//! - Discovery service types (DiscoveryRequest, DiscoveryResponse)
//! - Resource types (Cluster, Listener, Route, Endpoint, Secret)
//! - Configuration types
//! - gRPC service definitions
//!
//! ## Proto Generation
//!
//! Types are generated at build time from the Envoy data-plane-api protos.
//! The build script uses `tonic-build` and `prost-build` for code generation.
//!
//! ## Stub Types
//!
//! When proto files are not available (e.g., during initial development),
//! this crate provides stub types that match the expected API surface.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(missing_docs)] // Generated code doesn't have docs

// Re-export prost types for convenience
pub use prost::Message;
pub use prost_types::Any;

pub mod envoy {
    //! Envoy xDS types.

    pub mod service {
        //! Envoy discovery service definitions.

        pub mod discovery {
            //! Core discovery service types.

            pub mod v3 {
                //! Discovery service v3 API.

                /// Discovery request sent by clients.
                #[derive(Clone, PartialEq, Debug, Default)]
                pub struct DiscoveryRequest {
                    /// Version info from the last response (empty on first request).
                    pub version_info: String,
                    /// Node information.
                    pub node: Option<super::super::super::config::core::v3::Node>,
                    /// Requested resource names (empty for wildcard).
                    pub resource_names: Vec<String>,
                    /// Type URL of requested resources.
                    pub type_url: String,
                    /// Nonce from the last response.
                    pub response_nonce: String,
                    /// Error details if this is a NACK.
                    pub error_detail: Option<crate::google::rpc::Status>,
                }

                /// Discovery response sent by servers.
                #[derive(Clone, PartialEq, Debug, Default)]
                pub struct DiscoveryResponse {
                    /// Version of this response.
                    pub version_info: String,
                    /// Resources.
                    pub resources: Vec<prost_types::Any>,
                    /// Whether this is a canary response.
                    pub canary: bool,
                    /// Type URL of the resources.
                    pub type_url: String,
                    /// Unique nonce for this response.
                    pub nonce: String,
                    /// Control plane identifier.
                    pub control_plane: Option<super::super::super::config::core::v3::ControlPlane>,
                }

                /// Delta discovery request.
                #[derive(Clone, PartialEq, Debug, Default)]
                pub struct DeltaDiscoveryRequest {
                    /// Node information.
                    pub node: Option<super::super::super::config::core::v3::Node>,
                    /// Type URL of requested resources.
                    pub type_url: String,
                    /// Resources to subscribe to.
                    pub resource_names_subscribe: Vec<String>,
                    /// Resources to unsubscribe from.
                    pub resource_names_unsubscribe: Vec<String>,
                    /// Initial resource versions the client has.
                    pub initial_resource_versions: std::collections::HashMap<String, String>,
                    /// Nonce from the last response.
                    pub response_nonce: String,
                    /// Error details if this is a NACK.
                    pub error_detail: Option<crate::google::rpc::Status>,
                }

                /// Delta discovery response.
                #[derive(Clone, PartialEq, Debug, Default)]
                pub struct DeltaDiscoveryResponse {
                    /// System version info.
                    pub system_version_info: String,
                    /// Updated resources.
                    pub resources: Vec<Resource>,
                    /// Type URL of the resources.
                    pub type_url: String,
                    /// Removed resource names.
                    pub removed_resources: Vec<String>,
                    /// Unique nonce for this response.
                    pub nonce: String,
                    /// Control plane identifier.
                    pub control_plane: Option<super::super::super::config::core::v3::ControlPlane>,
                }

                /// A resource in a delta response.
                #[derive(Clone, PartialEq, Debug, Default)]
                pub struct Resource {
                    /// Resource name.
                    pub name: String,
                    /// Aliases for this resource.
                    pub aliases: Vec<String>,
                    /// Resource version.
                    pub version: String,
                    /// The resource.
                    pub resource: Option<prost_types::Any>,
                    /// TTL for this resource.
                    pub ttl: Option<prost_types::Duration>,
                    /// Cache control.
                    pub cache_control: Option<resource::CacheControl>,
                }

                pub mod resource {
                    //! Resource sub-types.

                    /// Cache control for a resource.
                    #[derive(Clone, PartialEq, Debug, Default)]
                    pub struct CacheControl {
                        /// Do not cache this resource.
                        pub do_not_cache: bool,
                    }
                }
            }
        }
    }

    pub mod config {
        //! Envoy configuration types.

        pub mod core {
            //! Core configuration types.

            pub mod v3 {
                //! Core v3 API.

                /// Node information.
                #[derive(Clone, PartialEq, Debug, Default)]
                pub struct Node {
                    /// Node identifier.
                    pub id: String,
                    /// Cluster the node belongs to.
                    pub cluster: String,
                    /// Node metadata.
                    pub metadata: Option<prost_types::Struct>,
                    /// Dynamic parameters.
                    pub dynamic_parameters: std::collections::HashMap<String, crate::xds::core::v3::ContextParams>,
                    /// Locality.
                    pub locality: Option<Locality>,
                    /// User agent name.
                    pub user_agent_name: String,
                    /// User agent version.
                    pub user_agent_version: Option<String>,
                    /// Extensions.
                    pub extensions: Vec<Extension>,
                    /// Client features.
                    pub client_features: Vec<String>,
                    /// Listening addresses.
                    pub listening_addresses: Vec<Address>,
                }

                /// Control plane identifier.
                #[derive(Clone, PartialEq, Debug, Default)]
                pub struct ControlPlane {
                    /// Identifier for this control plane.
                    pub identifier: String,
                }

                /// Locality information.
                #[derive(Clone, PartialEq, Debug, Default)]
                pub struct Locality {
                    /// Region.
                    pub region: String,
                    /// Zone.
                    pub zone: String,
                    /// Sub-zone.
                    pub sub_zone: String,
                }

                /// Extension information.
                #[derive(Clone, PartialEq, Debug, Default)]
                pub struct Extension {
                    /// Extension name.
                    pub name: String,
                    /// Category.
                    pub category: String,
                    /// Type descriptor.
                    pub type_descriptor: String,
                    /// Version.
                    pub version: Option<BuildVersion>,
                    /// Whether disabled.
                    pub disabled: bool,
                    /// Type URLs.
                    pub type_urls: Vec<String>,
                }

                /// Build version.
                #[derive(Clone, PartialEq, Debug, Default)]
                pub struct BuildVersion {
                    /// Version.
                    pub version: Option<SemanticVersion>,
                    /// Build metadata.
                    pub metadata: Option<prost_types::Struct>,
                }

                /// Semantic version.
                #[derive(Clone, PartialEq, Debug, Default)]
                pub struct SemanticVersion {
                    /// Major version.
                    pub major_number: u32,
                    /// Minor version.
                    pub minor_number: u32,
                    /// Patch version.
                    pub patch: u32,
                }

                /// Address.
                #[derive(Clone, PartialEq, Debug, Default)]
                pub struct Address {
                    /// Address type.
                    pub address: Option<address::Address>,
                }

                pub mod address {
                    //! Address types.

                    /// Address type enum.
                    #[derive(Clone, PartialEq, Debug)]
                    pub enum Address {
                        /// Socket address.
                        SocketAddress(super::SocketAddress),
                        /// Pipe address.
                        Pipe(super::Pipe),
                        /// Internal listener.
                        EnvoyInternalAddress(super::EnvoyInternalAddress),
                    }
                }

                /// Socket address.
                #[derive(Clone, PartialEq, Debug, Default)]
                pub struct SocketAddress {
                    /// Protocol.
                    pub protocol: i32,
                    /// Address.
                    pub address: String,
                    /// Port value.
                    pub port_value: u32,
                    /// Named port.
                    pub named_port: String,
                    /// Resolver name.
                    pub resolver_name: String,
                    /// IPv4 compat.
                    pub ipv4_compat: bool,
                }

                /// Pipe address.
                #[derive(Clone, PartialEq, Debug, Default)]
                pub struct Pipe {
                    /// Path.
                    pub path: String,
                    /// Mode.
                    pub mode: u32,
                }

                /// Envoy internal address.
                #[derive(Clone, PartialEq, Debug, Default)]
                pub struct EnvoyInternalAddress {
                    /// Server listener name.
                    pub server_listener_name: String,
                    /// Endpoint id.
                    pub endpoint_id: String,
                }
            }
        }

        pub mod cluster {
            //! Cluster configuration.

            pub mod v3 {
                //! Cluster v3 API.

                /// Cluster configuration.
                #[derive(Clone, PartialEq, Debug, Default)]
                pub struct Cluster {
                    /// Cluster name.
                    pub name: String,
                    // Additional fields omitted for brevity
                    // Full implementation would include all cluster config options
                }
            }
        }

        pub mod listener {
            //! Listener configuration.

            pub mod v3 {
                //! Listener v3 API.

                /// Listener configuration.
                #[derive(Clone, PartialEq, Debug, Default)]
                pub struct Listener {
                    /// Listener name.
                    pub name: String,
                    /// Address to listen on.
                    pub address: Option<super::super::core::v3::Address>,
                    // Additional fields omitted for brevity
                }
            }
        }

        pub mod route {
            //! Route configuration.

            pub mod v3 {
                //! Route v3 API.

                /// Route configuration.
                #[derive(Clone, PartialEq, Debug, Default)]
                pub struct RouteConfiguration {
                    /// Route config name.
                    pub name: String,
                    // Additional fields omitted for brevity
                }
            }
        }

        pub mod endpoint {
            //! Endpoint configuration.

            pub mod v3 {
                //! Endpoint v3 API.

                /// Cluster load assignment.
                #[derive(Clone, PartialEq, Debug, Default)]
                pub struct ClusterLoadAssignment {
                    /// Cluster name.
                    pub cluster_name: String,
                    /// Endpoints.
                    pub endpoints: Vec<LocalityLbEndpoints>,
                    // Additional fields omitted for brevity
                }

                /// Locality LB endpoints.
                #[derive(Clone, PartialEq, Debug, Default)]
                pub struct LocalityLbEndpoints {
                    /// Locality.
                    pub locality: Option<super::super::core::v3::Locality>,
                    /// Endpoints.
                    pub lb_endpoints: Vec<LbEndpoint>,
                    /// Load balancing weight.
                    pub load_balancing_weight: Option<u32>,
                    /// Priority.
                    pub priority: u32,
                }

                /// Load balancing endpoint.
                #[derive(Clone, PartialEq, Debug, Default)]
                pub struct LbEndpoint {
                    /// Health status.
                    pub health_status: i32,
                    /// Endpoint metadata.
                    pub metadata: Option<prost_types::Struct>,
                    /// Load balancing weight.
                    pub load_balancing_weight: Option<u32>,
                }
            }
        }
    }
}

pub mod google {
    //! Google API types.

    pub mod rpc {
        //! gRPC status types.

        /// Status type for error responses.
        #[derive(Clone, PartialEq, Debug, Default)]
        pub struct Status {
            /// Status code.
            pub code: i32,
            /// Status message.
            pub message: String,
            /// Details.
            pub details: Vec<prost_types::Any>,
        }
    }
}

pub mod xds {
    //! xDS core types.

    pub mod core {
        //! Core xDS types.

        pub mod v3 {
            //! xDS core v3 API.

            /// Context parameters for dynamic parameters.
            #[derive(Clone, PartialEq, Debug, Default)]
            pub struct ContextParams {
                /// Parameters.
                pub params: std::collections::HashMap<String, String>,
            }
        }
    }
}

/// Type URL constants.
pub mod type_url {
    /// Cluster type URL.
    pub const CLUSTER: &str = "type.googleapis.com/envoy.config.cluster.v3.Cluster";
    /// Listener type URL.
    pub const LISTENER: &str = "type.googleapis.com/envoy.config.listener.v3.Listener";
    /// Route type URL.
    pub const ROUTE: &str = "type.googleapis.com/envoy.config.route.v3.RouteConfiguration";
    /// Endpoint type URL.
    pub const ENDPOINT: &str = "type.googleapis.com/envoy.config.endpoint.v3.ClusterLoadAssignment";
    /// Secret type URL.
    pub const SECRET: &str = "type.googleapis.com/envoy.extensions.transport_sockets.tls.v3.Secret";
}
