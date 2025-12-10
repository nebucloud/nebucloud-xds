//! Domain Types for Logistics/Dispatch Control Plane
//!
//! This module demonstrates how to define domain-specific types
//! that will be converted to xDS resources.
//!
//! In a real application (like Dove Express), these would be your
//! business domain objects.

use serde::{Deserialize, Serialize};

/// A delivery load that needs to be assigned to a vehicle.
///
/// This represents the domain concept - not the xDS resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Load {
    /// Unique identifier for this load.
    pub id: String,
    /// Human-readable name (e.g., "Refrigerated cargo - Dallas to Houston").
    pub name: String,
    /// Origin coordinates.
    pub origin: Location,
    /// Destination coordinates.
    pub destination: Location,
    /// Weight in pounds.
    pub weight_lbs: u32,
    /// Required vehicle capabilities.
    pub requirements: Vec<String>,
    /// Priority level (1-10, higher is more urgent).
    pub priority: u8,
    /// Current status.
    pub status: LoadStatus,
}

/// A vehicle (truck) available for assignments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vehicle {
    /// Unique identifier.
    pub id: String,
    /// Driver name.
    pub driver_name: String,
    /// Current location.
    pub location: Location,
    /// Maximum weight capacity in pounds.
    pub capacity_lbs: u32,
    /// Vehicle capabilities (e.g., "refrigerated", "hazmat", "flatbed").
    pub capabilities: Vec<String>,
    /// Current status.
    pub status: VehicleStatus,
    /// Health score (0.0 - 1.0).
    pub health_score: f64,
}

/// An assignment of a load to a vehicle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assignment {
    /// Unique assignment ID.
    pub id: String,
    /// Load being assigned.
    pub load_id: String,
    /// Vehicle assigned to the load.
    pub vehicle_id: String,
    /// Estimated arrival time (ISO 8601).
    pub estimated_arrival: String,
    /// Route geometry (simplified).
    pub route_points: Vec<Location>,
    /// Assignment weight for load balancing.
    pub weight: u32,
}

/// Geographic location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
    pub name: Option<String>,
}

/// Load status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LoadStatus {
    /// Available for assignment.
    Available,
    /// Assigned to a vehicle.
    Assigned,
    /// Currently being transported.
    InTransit,
    /// Delivered.
    Delivered,
}

/// Vehicle status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VehicleStatus {
    /// Available for assignments.
    Available,
    /// Currently on a delivery.
    OnDelivery,
    /// Taking a required break.
    OnBreak,
    /// Out of service.
    Offline,
}

impl Load {
    /// Create a new load.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            origin: Location::default(),
            destination: Location::default(),
            weight_lbs: 0,
            requirements: vec![],
            priority: 5,
            status: LoadStatus::Available,
        }
    }

    /// Set origin location.
    pub fn origin(mut self, lat: f64, lng: f64, name: impl Into<String>) -> Self {
        self.origin = Location {
            latitude: lat,
            longitude: lng,
            name: Some(name.into()),
        };
        self
    }

    /// Set destination location.
    pub fn destination(mut self, lat: f64, lng: f64, name: impl Into<String>) -> Self {
        self.destination = Location {
            latitude: lat,
            longitude: lng,
            name: Some(name.into()),
        };
        self
    }

    /// Set weight.
    pub fn weight(mut self, lbs: u32) -> Self {
        self.weight_lbs = lbs;
        self
    }

    /// Add a requirement.
    pub fn require(mut self, capability: impl Into<String>) -> Self {
        self.requirements.push(capability.into());
        self
    }

    /// Set priority.
    pub fn priority(mut self, priority: u8) -> Self {
        self.priority = priority.clamp(1, 10);
        self
    }
}

impl Vehicle {
    /// Create a new vehicle.
    pub fn new(id: impl Into<String>, driver: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            driver_name: driver.into(),
            location: Location::default(),
            capacity_lbs: 45000, // Standard semi-truck capacity
            capabilities: vec![],
            status: VehicleStatus::Available,
            health_score: 1.0,
        }
    }

    /// Set current location.
    pub fn at(mut self, lat: f64, lng: f64) -> Self {
        self.location = Location {
            latitude: lat,
            longitude: lng,
            name: None,
        };
        self
    }

    /// Set capacity.
    pub fn capacity(mut self, lbs: u32) -> Self {
        self.capacity_lbs = lbs;
        self
    }

    /// Add a capability.
    pub fn with_capability(mut self, cap: impl Into<String>) -> Self {
        self.capabilities.push(cap.into());
        self
    }

    /// Set health score.
    pub fn health(mut self, score: f64) -> Self {
        self.health_score = score.clamp(0.0, 1.0);
        self
    }
}

impl Default for Location {
    fn default() -> Self {
        Self {
            latitude: 0.0,
            longitude: 0.0,
            name: None,
        }
    }
}
