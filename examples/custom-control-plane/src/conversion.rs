//! Conversion from Domain Types to xDS Resources
//!
//! This module demonstrates the key pattern for building a custom control plane:
//! converting your domain-specific types into standard xDS resources.
//!
//! ## Design Pattern
//!
//! The conversion layer is the bridge between your business logic and the xDS protocol.
//! It allows you to:
//!
//! 1. Keep domain logic clean and framework-agnostic
//! 2. Map domain concepts to xDS resources
//! 3. Evolve your domain model independently of xDS
//!
//! ## xDS Resource Mapping
//!
//! For a logistics/dispatch use case:
//!
//! | Domain Concept | xDS Resource | Purpose |
//! |----------------|--------------|---------|
//! | Load Type/Region | Cluster | Group of similar loads |
//! | Available Vehicles | ClusterLoadAssignment | Endpoints with capacity |
//! | Load→Vehicle Assignment | RouteConfiguration | Routing rules |
//! | Depot/Region | Listener | Entry point |

use crate::domain::{Assignment, Load, Vehicle, VehicleStatus};
use nebucloud_xds::prelude::*;
use std::any::Any;
use std::sync::Arc;
use xds_types::envoy::config::core::v3::Locality;

// ============================================================================
// Wrapped Resource Types
// ============================================================================
//
// Since xds-types are stub types that don't implement Resource,
// we create wrapper types that implement the Resource trait.
// This is the pattern external consumers would use.

/// A Cluster resource wrapper that implements Resource trait.
#[derive(Debug, Clone)]
pub struct ClusterResource {
    /// Cluster name (used as resource name).
    pub name: String,
    /// Original load ID this cluster represents.
    pub load_id: Option<String>,
    /// Load requirements/category.
    pub category: String,
}

impl Resource for ClusterResource {
    fn type_url(&self) -> &str {
        TypeUrl::CLUSTER
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn encode(&self) -> Result<prost_types::Any, Box<dyn std::error::Error + Send + Sync>> {
        // In production, you'd encode to actual protobuf
        Ok(prost_types::Any {
            type_url: self.type_url().to_string(),
            value: self.name.as_bytes().to_vec(), // Simplified
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An Endpoint resource wrapper for vehicle fleet assignments.
#[derive(Debug, Clone)]
pub struct EndpointResource {
    /// Cluster this endpoint belongs to.
    pub cluster_name: String,
    /// Vehicles in this cluster.
    pub vehicles: Vec<VehicleEndpoint>,
}

/// A single vehicle as an endpoint.
#[derive(Debug, Clone)]
pub struct VehicleEndpoint {
    /// Vehicle ID.
    pub id: String,
    /// Health status (1=healthy, 2=degraded, 3=timeout, 4=unhealthy).
    pub health_status: i32,
    /// Load balancing weight (based on capacity and health).
    pub weight: u32,
    /// Locality (region/zone).
    pub locality: Locality,
}

impl Resource for EndpointResource {
    fn type_url(&self) -> &str {
        TypeUrl::ENDPOINT
    }

    fn name(&self) -> &str {
        &self.cluster_name
    }

    fn encode(&self) -> Result<prost_types::Any, Box<dyn std::error::Error + Send + Sync>> {
        Ok(prost_types::Any {
            type_url: self.type_url().to_string(),
            value: self.cluster_name.as_bytes().to_vec(), // Simplified
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A Route resource wrapper for load assignments.
#[derive(Debug, Clone)]
pub struct RouteResource {
    /// Route configuration name.
    pub name: String,
    /// Assignment this route represents.
    pub assignment_id: String,
    /// Target vehicle cluster.
    pub target_cluster: String,
}

impl Resource for RouteResource {
    fn type_url(&self) -> &str {
        TypeUrl::ROUTE
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn encode(&self) -> Result<prost_types::Any, Box<dyn std::error::Error + Send + Sync>> {
        Ok(prost_types::Any {
            type_url: self.type_url().to_string(),
            value: self.name.as_bytes().to_vec(), // Simplified
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// ============================================================================
// Conversion Functions
// ============================================================================

/// Convert a Load to a ClusterResource.
pub fn load_to_cluster(load: &Load) -> ClusterResource {
    let category = if load.requirements.is_empty() {
        "general".to_string()
    } else {
        load.requirements.join("-")
    };

    ClusterResource {
        name: format!("load-{}", load.id),
        load_id: Some(load.id.clone()),
        category,
    }
}

/// Create a cluster for a load category (e.g., "refrigerated-midwest").
pub fn category_to_cluster(category: &str) -> ClusterResource {
    ClusterResource {
        name: format!("category-{}", category),
        load_id: None,
        category: category.to_string(),
    }
}

/// Convert a Vehicle to a VehicleEndpoint.
pub fn vehicle_to_endpoint(vehicle: &Vehicle) -> VehicleEndpoint {
    VehicleEndpoint {
        id: vehicle.id.clone(),
        health_status: match vehicle.status {
            VehicleStatus::Available => 1,    // HEALTHY
            VehicleStatus::OnDelivery => 2,   // DEGRADED (busy but reachable)
            VehicleStatus::OnBreak => 3,      // TIMEOUT
            VehicleStatus::Offline => 4,      // UNHEALTHY
        },
        weight: (vehicle.health_score * 100.0) as u32,
        locality: vehicle_to_locality(vehicle),
    }
}

/// Convert vehicle location to locality.
pub fn vehicle_to_locality(vehicle: &Vehicle) -> Locality {
    let (region, zone) = gps_to_region(vehicle.location.latitude, vehicle.location.longitude);
    Locality {
        region,
        zone,
        sub_zone: String::new(),
    }
}

/// Convert vehicles to an EndpointResource.
pub fn vehicles_to_endpoint_resource(cluster_name: &str, vehicles: &[Vehicle]) -> EndpointResource {
    let endpoints: Vec<VehicleEndpoint> = vehicles
        .iter()
        .filter(|v| v.status != VehicleStatus::Offline)
        .map(vehicle_to_endpoint)
        .collect();

    EndpointResource {
        cluster_name: cluster_name.to_string(),
        vehicles: endpoints,
    }
}

/// Convert an Assignment to a RouteResource.
pub fn assignment_to_route(assignment: &Assignment) -> RouteResource {
    RouteResource {
        name: format!("assignment-{}", assignment.id),
        assignment_id: assignment.id.clone(),
        target_cluster: format!("vehicle-{}", assignment.vehicle_id),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Map GPS coordinates to region/zone.
pub fn gps_to_region(lat: f64, lng: f64) -> (String, String) {
    // Simplified US region mapping
    let region = if lng < -100.0 {
        "us-west"
    } else if lng < -85.0 {
        "us-central"
    } else {
        "us-east"
    };

    let zone = if lat > 40.0 {
        "north"
    } else if lat > 32.0 {
        "central"
    } else {
        "south"
    };

    (region.to_string(), format!("{}-{}", region, zone))
}

/// Wrap a resource in an Arc for use as BoxResource.
pub fn boxed<R: Resource + 'static>(resource: R) -> BoxResource {
    Arc::new(resource)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Load, Vehicle};

    #[test]
    fn test_load_to_cluster() {
        let load = Load::new("load-001", "Refrigerated cargo")
            .origin(32.7767, -96.7970, "Dallas, TX")
            .destination(29.7604, -95.3698, "Houston, TX")
            .weight(20000)
            .require("refrigerated");

        let cluster = load_to_cluster(&load);
        assert_eq!(cluster.name, "load-load-001");
        assert_eq!(cluster.category, "refrigerated");
    }

    #[test]
    fn test_vehicle_to_endpoint() {
        let vehicle = Vehicle::new("truck-001", "John Doe")
            .at(32.7767, -96.7970)
            .capacity(45000)
            .with_capability("refrigerated")
            .health(0.95);

        let endpoint = vehicle_to_endpoint(&vehicle);
        assert_eq!(endpoint.id, "truck-001");
        assert_eq!(endpoint.health_status, 1); // HEALTHY
        assert_eq!(endpoint.weight, 95);
    }

    #[test]
    fn test_gps_to_region() {
        // Dallas, TX
        let (region, zone) = gps_to_region(32.7767, -96.7970);
        assert_eq!(region, "us-central");
        assert!(zone.contains("central"));

        // Los Angeles, CA
        let (region, _zone) = gps_to_region(34.0522, -118.2437);
        assert_eq!(region, "us-west");
    }
}
