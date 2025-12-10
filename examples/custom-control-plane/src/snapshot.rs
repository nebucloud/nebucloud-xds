//! Snapshot Management for Domain State
//!
//! This module demonstrates how to build and manage xDS snapshots
//! from your domain state. The key concept is that whenever your
//! domain state changes (new loads, vehicle updates, assignments),
//! you rebuild and push a new snapshot.
//!
//! ## Snapshot Lifecycle
//!
//! ```text
//! Domain Event (new load, vehicle update, etc.)
//!        │
//!        ▼
//! ┌─────────────────────┐
//! │  Update Domain State │
//! └─────────────────────┘
//!        │
//!        ▼
//! ┌─────────────────────┐
//! │  Build New Snapshot  │  ← This module
//! └─────────────────────┘
//!        │
//!        ▼
//! ┌─────────────────────┐
//! │  Push to Cache       │
//! └─────────────────────┘
//!        │
//!        ▼
//! ┌─────────────────────┐
//! │  xDS Notifies Envoy  │  ← Automatic via nebucloud-xds
//! └─────────────────────┘
//! ```

use crate::conversion::{
    assignment_to_route, boxed, category_to_cluster, gps_to_region,
    vehicles_to_endpoint_resource, ClusterResource, EndpointResource, RouteResource,
};
use crate::domain::{Assignment, Load, LoadStatus, Vehicle, VehicleStatus};
use nebucloud_xds::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Manages domain state and generates xDS snapshots.
///
/// This is the central coordinator for your control plane.
/// It maintains the current state and knows how to convert
/// it to xDS snapshots for different node types.
pub struct DispatchState {
    /// Current version counter.
    version: AtomicU64,
    /// Active loads.
    loads: HashMap<String, Load>,
    /// Available vehicles.
    vehicles: HashMap<String, Vehicle>,
    /// Current assignments.
    assignments: HashMap<String, Assignment>,
}

impl DispatchState {
    /// Create a new dispatch state manager.
    pub fn new() -> Self {
        Self {
            version: AtomicU64::new(0),
            loads: HashMap::new(),
            vehicles: HashMap::new(),
            assignments: HashMap::new(),
        }
    }

    /// Get the next version string.
    fn next_version(&self) -> String {
        let v = self.version.fetch_add(1, Ordering::SeqCst);
        format!("v{}", v + 1)
    }

    // ========================================================================
    // Domain State Mutations
    // ========================================================================

    /// Add or update a load.
    pub fn upsert_load(&mut self, load: Load) {
        self.loads.insert(load.id.clone(), load);
    }

    /// Remove a load.
    #[allow(dead_code)]
    pub fn remove_load(&mut self, load_id: &str) {
        self.loads.remove(load_id);
    }

    /// Add or update a vehicle.
    pub fn upsert_vehicle(&mut self, vehicle: Vehicle) {
        self.vehicles.insert(vehicle.id.clone(), vehicle);
    }

    /// Remove a vehicle.
    #[allow(dead_code)]
    pub fn remove_vehicle(&mut self, vehicle_id: &str) {
        self.vehicles.remove(vehicle_id);
    }

    /// Add an assignment.
    pub fn add_assignment(&mut self, assignment: Assignment) {
        // Update load status
        if let Some(load) = self.loads.get_mut(&assignment.load_id) {
            load.status = LoadStatus::Assigned;
        }
        // Update vehicle status
        if let Some(vehicle) = self.vehicles.get_mut(&assignment.vehicle_id) {
            vehicle.status = VehicleStatus::OnDelivery;
        }
        self.assignments.insert(assignment.id.clone(), assignment);
    }

    /// Complete an assignment.
    #[allow(dead_code)]
    pub fn complete_assignment(&mut self, assignment_id: &str) {
        if let Some(assignment) = self.assignments.remove(assignment_id) {
            // Update load status
            if let Some(load) = self.loads.get_mut(&assignment.load_id) {
                load.status = LoadStatus::Delivered;
            }
            // Update vehicle status
            if let Some(vehicle) = self.vehicles.get_mut(&assignment.vehicle_id) {
                vehicle.status = VehicleStatus::Available;
            }
        }
    }

    // ========================================================================
    // Snapshot Generation
    // ========================================================================

    /// Build a snapshot for dispatcher nodes.
    ///
    /// Dispatcher nodes need to see:
    /// - All available loads (as clusters)
    /// - All available vehicles (as endpoints)
    /// - Current assignments (as routes)
    pub fn build_dispatcher_snapshot(&self) -> Snapshot {
        let version = self.next_version();

        // Group loads by category for efficient clustering
        let available_loads: Vec<_> = self
            .loads
            .values()
            .filter(|l| l.status == LoadStatus::Available)
            .collect();

        // Group vehicles by capability
        let available_vehicles: Vec<_> = self
            .vehicles
            .values()
            .filter(|v| v.status == VehicleStatus::Available)
            .cloned()
            .collect();

        // Build clusters from load categories
        let clusters = self.build_load_clusters(&available_loads);

        // Build endpoints from vehicles
        let endpoints = self.build_vehicle_endpoints(&available_vehicles);

        // Build routes from assignments
        let routes = self.build_assignment_routes();

        tracing::info!(
            version = %version,
            clusters = clusters.len(),
            endpoints = endpoints.len(),
            routes = routes.len(),
            "Built dispatcher snapshot"
        );

        Snapshot::builder()
            .version(&version)
            .resources(TypeUrl::new(TypeUrl::CLUSTER), clusters)
            .resources(TypeUrl::new(TypeUrl::ENDPOINT), endpoints)
            .resources(TypeUrl::new(TypeUrl::ROUTE), routes)
            .build()
    }

    /// Build a snapshot for vehicle/driver nodes.
    ///
    /// Vehicle nodes only need to see:
    /// - Their assigned loads
    /// - Route to dispatch API
    pub fn build_vehicle_snapshot(&self, vehicle_id: &str) -> Snapshot {
        let version = self.next_version();

        // Find assignments for this vehicle
        let my_assignments: Vec<_> = self
            .assignments
            .values()
            .filter(|a| a.vehicle_id == vehicle_id)
            .collect();

        // Build routes for assigned loads
        let routes: Vec<BoxResource> = my_assignments
            .iter()
            .map(|a| boxed(assignment_to_route(a)))
            .collect();

        // Cluster for dispatch API
        let dispatch_cluster = boxed(ClusterResource {
            name: "dispatch-api".to_string(),
            load_id: None,
            category: "api".to_string(),
        });

        tracing::debug!(
            vehicle_id = %vehicle_id,
            version = %version,
            assignments = my_assignments.len(),
            "Built vehicle snapshot"
        );

        Snapshot::builder()
            .version(&version)
            .resources(TypeUrl::new(TypeUrl::CLUSTER), vec![dispatch_cluster])
            .resources(TypeUrl::new(TypeUrl::ROUTE), routes)
            .build()
    }

    /// Build a snapshot for depot/regional nodes.
    ///
    /// Depot nodes need to see:
    /// - Vehicles in their region (endpoints)
    /// - Regional load clusters
    pub fn build_depot_snapshot(&self, region: &str) -> Snapshot {
        let version = self.next_version();

        // Filter vehicles by region
        let regional_vehicles: Vec<_> = self
            .vehicles
            .values()
            .filter(|v| {
                let (veh_region, _) =
                    gps_to_region(v.location.latitude, v.location.longitude);
                veh_region == region
            })
            .cloned()
            .collect();

        // Build endpoints for regional vehicles
        let endpoints: Vec<BoxResource> = if regional_vehicles.is_empty() {
            vec![]
        } else {
            let endpoint = vehicles_to_endpoint_resource(
                &format!("fleet-{}", region),
                &regional_vehicles,
            );
            vec![boxed(endpoint)]
        };

        tracing::debug!(
            region = %region,
            version = %version,
            vehicles = regional_vehicles.len(),
            "Built depot snapshot"
        );

        Snapshot::builder()
            .version(&version)
            .resources(TypeUrl::new(TypeUrl::ENDPOINT), endpoints)
            .build()
    }

    // ========================================================================
    // Internal Helpers
    // ========================================================================

    fn build_load_clusters(&self, loads: &[&Load]) -> Vec<BoxResource> {
        // Group by requirements
        let mut by_category: HashMap<String, Vec<&Load>> = HashMap::new();

        for load in loads {
            let category = if load.requirements.is_empty() {
                "general".to_string()
            } else {
                load.requirements.join("-")
            };
            by_category.entry(category).or_default().push(load);
        }

        by_category
            .into_keys()
            .map(|category| boxed(category_to_cluster(&category)))
            .collect()
    }

    fn build_vehicle_endpoints(&self, vehicles: &[Vehicle]) -> Vec<BoxResource> {
        if vehicles.is_empty() {
            return vec![];
        }

        // Group by capability for separate endpoint groups
        let mut by_capability: HashMap<String, Vec<Vehicle>> = HashMap::new();

        for vehicle in vehicles {
            let cap = if vehicle.capabilities.is_empty() {
                "general".to_string()
            } else {
                vehicle.capabilities.join("-")
            };
            by_capability.entry(cap).or_default().push(vehicle.clone());
        }

        by_capability
            .into_iter()
            .map(|(cap, vehicles)| {
                let endpoint = vehicles_to_endpoint_resource(&format!("vehicles-{}", cap), &vehicles);
                boxed(endpoint)
            })
            .collect()
    }

    fn build_assignment_routes(&self) -> Vec<BoxResource> {
        self.assignments
            .values()
            .map(|a| boxed(assignment_to_route(a)))
            .collect()
    }
}

impl Default for DispatchState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Load, Vehicle};

    #[test]
    fn test_build_dispatcher_snapshot() {
        let mut state = DispatchState::new();

        // Add some loads
        state.upsert_load(
            Load::new("load-001", "Refrigerated cargo")
                .origin(32.7767, -96.7970, "Dallas")
                .destination(29.7604, -95.3698, "Houston")
                .require("refrigerated"),
        );

        // Add some vehicles
        state.upsert_vehicle(
            Vehicle::new("truck-001", "John Doe")
                .at(32.7767, -96.7970)
                .with_capability("refrigerated"),
        );

        let snapshot = state.build_dispatcher_snapshot();
        assert_eq!(snapshot.version(), "v1");
    }

    #[test]
    fn test_version_increments() {
        let state = DispatchState::new();

        let s1 = state.build_dispatcher_snapshot();
        let s2 = state.build_dispatcher_snapshot();

        assert_eq!(s1.version(), "v1");
        assert_eq!(s2.version(), "v2");
    }
}
