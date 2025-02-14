//! The convert_osm crate produces a RawMap from OSM and other data. Storing this intermediate
//! structure is useful to iterate quickly on parts of the map importing pipeline without having to
//! constantly read .osm files, and to visualize the intermediate state with map_editor.

use std::collections::BTreeMap;

use osm2streets::{osm, StreetNetwork};
use serde::{Deserialize, Serialize};

use abstio::{CityName, MapName};
use abstutil::{
    deserialize_btreemap, deserialize_multimap, serialize_btreemap, serialize_multimap, MultiMap,
    Tags,
};
use geom::{PolyLine, Polygon, Pt2D};

pub use self::types::{Amenity, AmenityType, AreaType};

mod types;

#[derive(Serialize, Deserialize)]
pub struct RawMap {
    pub name: MapName,
    pub streets: StreetNetwork,
    #[serde(
        serialize_with = "serialize_btreemap",
        deserialize_with = "deserialize_btreemap"
    )]
    pub buildings: BTreeMap<osm::OsmID, RawBuilding>,
    pub areas: Vec<RawArea>,
    pub parking_lots: Vec<RawParkingLot>,
    pub parking_aisles: Vec<(osm::WayID, Vec<Pt2D>)>,
    pub transit_routes: Vec<RawTransitRoute>,
    #[serde(
        serialize_with = "serialize_btreemap",
        deserialize_with = "deserialize_btreemap"
    )]
    pub transit_stops: BTreeMap<String, RawTransitStop>,
    /// Per road, what bus routes run along it?
    ///
    /// This is scraped from OSM relations for every map, unlike the more detailed `transit_routes`
    /// above, which come from GTFS only for a few maps. This is used only to identify roads part
    /// of bus routes. It's best-effort and not robust to edits or transformations.
    #[serde(
        serialize_with = "serialize_multimap",
        deserialize_with = "deserialize_multimap"
    )]
    pub bus_routes_on_roads: MultiMap<osm::WayID, String>,
}

impl RawMap {
    pub fn blank(name: MapName) -> RawMap {
        RawMap {
            name,
            streets: StreetNetwork::blank(),
            buildings: BTreeMap::new(),
            areas: Vec::new(),
            parking_lots: Vec::new(),
            parking_aisles: Vec::new(),
            transit_routes: Vec::new(),
            transit_stops: BTreeMap::new(),
            bus_routes_on_roads: MultiMap::new(),
        }
    }

    // TODO Almost gone...
    pub fn new_osm_way_id(&self, start: i64) -> osm::WayID {
        assert!(start < 0);
        // Slow, but deterministic.
        let mut osm_way_id = start;
        loop {
            let candidate = osm::WayID(osm_way_id);
            // TODO Doesn't handle collisions with areas or parking lots
            if self
                .streets
                .roads
                .keys()
                .any(|r| r.osm_way_id.0 == osm_way_id)
                || self
                    .buildings
                    .keys()
                    .any(|b| b == &osm::OsmID::Way(candidate))
            {
                osm_way_id -= 1;
            } else {
                return candidate;
            }
        }
    }

    pub fn new_osm_node_id(&self, start: i64) -> osm::NodeID {
        assert!(start < 0);
        // Slow, but deterministic.
        let mut osm_node_id = start;
        loop {
            if self
                .streets
                .intersections
                .keys()
                .any(|i| i.0 == osm_node_id)
            {
                osm_node_id -= 1;
            } else {
                return osm::NodeID(osm_node_id);
            }
        }
    }

    pub fn save(&self) {
        abstio::write_binary(abstio::path_raw_map(&self.name), self)
    }

    pub fn get_city_name(&self) -> &CityName {
        &self.name.city
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawBuilding {
    pub polygon: Polygon,
    pub osm_tags: Tags,
    pub public_garage_name: Option<String>,
    pub num_parking_spots: usize,
    pub amenities: Vec<Amenity>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawArea {
    pub area_type: AreaType,
    pub polygon: Polygon,
    pub osm_tags: Tags,
    pub osm_id: osm::OsmID,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawParkingLot {
    pub osm_id: osm::OsmID,
    pub polygon: Polygon,
    pub osm_tags: Tags,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawTransitRoute {
    pub long_name: String,
    pub short_name: String,
    pub gtfs_id: String,
    /// This may begin and/or end inside or outside the map boundary.
    pub shape: PolyLine,
    /// Entries into transit_stops
    pub stops: Vec<String>,
    pub route_type: RawTransitType,
    // TODO Schedule
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RawTransitType {
    Bus,
    Train,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawTransitStop {
    pub gtfs_id: String,
    /// Only stops within a map's boundary are kept
    pub position: Pt2D,
    pub name: String,
}
