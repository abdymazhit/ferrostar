pub(crate) mod models;

use super::RouteResponseParser;
use crate::models::{GeographicCoordinates, RouteStep};
use crate::routing_adapters::{osrm::models::RouteResponse, Route, RoutingResponseParseError};
use crate::RoutingResponseParseError::ParseError;
use polyline::decode_polyline;

/// A response parser for OSRM-compatible routing backends.
///
/// The parser is NOT limited to only the standard OSRM format; many Valhalla/Mapbox tags are also
/// parsed and are included in the final route.
#[derive(Debug)]
pub struct OsrmResponseParser {
    polyline_precision: u32,
}

impl OsrmResponseParser {
    pub fn new(polyline_precision: u32) -> Self {
        Self { polyline_precision }
    }
}

impl RouteResponseParser for OsrmResponseParser {
    fn parse_response(&self, response: Vec<u8>) -> Result<Vec<Route>, RoutingResponseParseError> {
        let res: RouteResponse = serde_json::from_slice(&response)?;
        let waypoints: Vec<_> = res
            .waypoints
            .iter()
            .map(|waypoint| GeographicCoordinates {
                lat: waypoint.location.latitude(),
                lng: waypoint.location.longitude(),
            })
            .collect();

        // This isn't the most functional in style, but it's a bit difficult to construct a pipeline
        // today. Stabilization of try_collect may help.
        let mut routes = vec![];
        for route in res.routes {
            let geometry = decode_polyline(&route.geometry, self.polyline_precision)
                .map_err(|error| RoutingResponseParseError::ParseError {
                    error: error.clone(),
                })?
                .coords()
                .map(|coord| GeographicCoordinates::from(*coord))
                .collect();

            let mut steps = vec![];
            for leg in route.legs {
                for step in leg.steps {
                    steps.push(RouteStep::from_osrm(&step, self.polyline_precision)?);
                }
            }

            routes.push(Route {
                geometry,
                waypoints: waypoints.clone(),
                steps,
            })
        }

        Ok(routes)
    }
}

impl RouteStep {
    fn from_osrm(
        value: &models::RouteStep,
        polyline_precision: u32,
    ) -> Result<Self, RoutingResponseParseError> {
        let linestring = decode_polyline(&value.geometry, polyline_precision)
            .map_err(|error| RoutingResponseParseError::ParseError { error })?;
        let mut geometry = linestring.coords();

        let start_location = geometry
            .next()
            .map(|coord| GeographicCoordinates::from(*coord))
            .ok_or(ParseError {
                error: "No coordinates in geometry".to_string(),
            })?;
        let end_location = geometry
            .last()
            .map_or(start_location, |coord| GeographicCoordinates::from(*coord));

        Ok(RouteStep {
            start_location,
            end_location,
            distance: value.distance,
            road_name: value.name.clone(),
            instruction: value.maneuver.get_instruction(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STANDARD_OSRM_POLYLINE6_RESPONSE: &str = r#"{"code":"Ok","routes":[{"geometry":"qikdcB{~dpXmxRbaBuqAoqKyy@svFwNcfKzsAysMdr@evD`m@qrAohBi}A{OkdGjg@ajDZww@lJ}Jrs@}`CvzBq`E`PiB`~A|l@z@feA","legs":[{"steps":[],"summary":"","weight":263.1,"duration":260.2,"distance":1886.3},{"steps":[],"summary":"","weight":370.5,"duration":370.5,"distance":2845.5}],"weight_name":"routability","weight":633.6,"duration":630.7,"distance":4731.8}],"waypoints":[{"hint":"Dv8JgCp3moUXAAAABQAAAAAAAAAgAAAAIXRPQYXNK0AAAAAAcPePQQsAAAADAAAAAAAAABAAAAA6-wAA_kvMAKlYIQM8TMwArVghAwAA7wrXLH_K","distance":4.231521214,"name":"Friedrichstraße","location":[13.388798,52.517033]},{"hint":"JEvdgVmFiocGAAAACgAAAAAAAAB3AAAAppONQOodwkAAAAAA8TeEQgYAAAAKAAAAAAAAAHcAAAA6-wAAfm7MABiJIQOCbswA_4ghAwAAXwXXLH_K","distance":2.795148358,"name":"Torstraße","location":[13.39763,52.529432]},{"hint":"oSkYgP___38fAAAAUQAAACYAAAAeAAAAeosKQlNOX0IQ7CZCjsMGQh8AAABRAAAAJgAAAB4AAAA6-wAASufMAOdwIQNL58wA03AhAwQAvxDXLH_K","distance":2.226580806,"name":"Platz der Vereinten Nationen","location":[13.428554,52.523239]}]}"#;
    // TODO: Include banners
    const VALHALLA_OSRM_RESPONSE: &str = r#"{"code":"Ok","routes":[{"distance":2604.35,"duration":2007.289,"geometry":"e|akpBozpfn@AG~ApSAzFg@pKsFvfA]lFdDr@kAvOoAvDkC|B]~DMzAyCj^c@lFi@d@wIbHu@f@cV|PkA~@_TxQxX|eC{Az@qDrBw@b@{BnATbCNjBd@rHyAj@g@JiDrAcJxDcBjBcA^sDvAsIjDmCnD}@R`@bHgHnBsRvGkDhCsDpTpF~dEPfMfAft@H~FNrEdAt}@f@pY@rA?`@@rBJhRCdAIbD]nFa@bDaAbIiAdImB~MKt@wGrd@qBnOoDbUwAxJVfH\\jMHpEGzAiAjDqMbf@gBnFkC~HeDbKs@vBkCtF}CpGuIzNU`@oGzH{FhGqi@hc@ud@t_@wIpI{JfNqLfTwJjVgDdJ_HvYaEpUgHxa@aFhd@mErt@q@~FmFrd@oJdw@kFmDsCyAyArJgAdAJhDm@`G_@fCMrAmAfFiKf|@{Fxh@oCdSi@dGaBrQcBbNwCd\\kGlh@uA~PuEzr@_@bHa@dC}@\\KbEOvCk@FoQbw@uNno@Gv@SxCo@hEiA`@i@nBf@pCQtDk@xC{B|KgTraAuA\\i@o@mFzY}GiGqBoC","legs":[{"admins":[{"iso_3166_1":"EE","iso_3166_1_alpha3":"EST"}],"annotation":{"distance":[0.2,19.4,7.1,11.6,66.4,6.9,9.4,15.7,6.9,8.6,5.7,2.7,29.7,7.0,2.6,20.9,3.2,44.3,4.6,41.1,130.5,5.4,10.4,3.3,7.3,3.9,3.2,9.0,5.2,2.3,9.8,20.5,6.4,3.9,10.3,19.5,9.3,3.5,8.5,16.8,35.8,10.3,21.9,179.8,12.9,48.4,7.3,6.1,56.9,24.2,2.4,1.0,3.3,17.5,2.0,4.7,7.0,5.0,9.9,10.1,14.9,1.7,37.5,16.2,22.3,11.8,8.5,13.1,6.0,2.6,6.4,43.9,8.9,11.9,14.3,4.5,10.4,11.7,23.9,1.6,17.6,15.9,82.6,73.4,21.4,25.3,30.9,29.8,13.8,29.0,23.1,35.6,36.0,49.9,7.8,36.5,54.8,14.0,8.6,11.7,4.5,4.9,7.7,4.2,2.5,7.9,59.6,40.4,20.0,7.8,17.7,14.8,27.7,40.4,17.0,48.4,8.5,4.2,3.6,5.6,4.4,2.5,60.6,52.0,1.6,4.5,6.3,4.2,3.9,4.7,5.2,5.0,13.6,71.2,4.9,2.7,27.7,17.6,7.5],"duration":[0.184,15.315,5.639,9.818,51.539,4.898,6.604,12.227,5.127,6.412,4.012,1.919,20.947,4.96,1.815,14.72,2.267,33.128,3.248,29.012,101.367,3.808,8.841,2.592,5.742,2.774,2.247,6.331,3.643,1.589,6.887,14.472,4.482,2.747,7.288,13.793,6.594,2.469,5.983,13.027,27.83,8.028,15.49,126.908,12.15,34.152,5.129,4.281,42.571,18.073,1.781,0.681,2.318,17.664,2.012,4.717,7.059,5.058,7.669,7.844,10.516,1.177,26.445,11.458,15.741,8.304,5.987,13.246,4.213,1.864,4.77,32.852,6.677,8.938,10.737,3.339,7.818,11.006,22.394,1.32,14.893,13.483,69.994,51.784,15.108,19.969,24.415,23.531,10.899,32.187,16.31,25.104,25.445,35.213,5.477,25.799,38.709,9.902,6.086,8.228,3.156,3.427,5.46,2.993,1.871,5.888,44.617,30.206,15.496,5.487,12.51,10.434,19.585,31.347,13.188,37.62,6.681,3.349,2.809,3.942,3.1,1.737,45.312,40.41,1.278,3.174,4.898,3.284,3.057,3.644,4.072,3.527,13.723,50.263,3.432,1.908,20.727,17.401,7.403],"maxspeed":[{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"speed":30,"unit":"km/h"},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true},{"unknown":true}],"speed":[1.3,1.3,1.3,1.2,1.3,1.4,1.4,1.3,1.3,1.3,1.4,1.4,1.4,1.4,1.4,1.4,1.4,1.3,1.4,1.4,1.3,1.4,1.2,1.3,1.3,1.4,1.4,1.4,1.4,1.4,1.4,1.4,1.4,1.4,1.4,1.4,1.4,1.4,1.4,1.3,1.3,1.3,1.4,1.4,1.1,1.4,1.4,1.4,1.3,1.3,1.3,1.4,1.4,1.0,1.0,1.0,1.0,1.0,1.3,1.3,1.4,1.4,1.4,1.4,1.4,1.4,1.4,1.0,1.4,1.4,1.3,1.3,1.3,1.3,1.3,1.3,1.3,1.1,1.1,1.2,1.2,1.2,1.2,1.4,1.4,1.3,1.3,1.3,1.3,0.9,1.4,1.4,1.4,1.4,1.4,1.4,1.4,1.4,1.4,1.4,1.4,1.4,1.4,1.4,1.3,1.3,1.3,1.3,1.3,1.4,1.4,1.4,1.4,1.3,1.3,1.3,1.3,1.3,1.3,1.4,1.4,1.4,1.3,1.3,1.3,1.4,1.3,1.3,1.3,1.3,1.3,1.4,1.0,1.4,1.4,1.4,1.3,1.0,1.0]},"distance":2604.35,"duration":2007.289,"steps":[{"bannerInstructions":[{"distanceAlongGeometry":111.251,"primary":{"components":[{"text":"Turn left onto the walkway.","type":"text"}],"modifier":"left","text":"Turn left onto the walkway.","type":"turn"}}],"distance":111.251,"driving_side":"right","duration":90.107,"geometry":"e|akpBozpfn@AG~ApSAzFg@pKsFvfA]lF","intersections":[{"admin_index":0,"bearings":[254],"duration":20.754,"entry":[true],"geometry_index":0,"location":[24.765368,59.442643],"out":0,"weight":21.791},{"admin_index":0,"bearings":[7,82,189,281],"duration":11.165,"entry":[true,false,true,true],"geometry_index":3,"in":1,"location":[24.764917,59.442597],"out":3,"turn_duration":1.0,"turn_weight":1.0,"weight":12.181},{"admin_index":0,"bearings":[13,101,191,282],"duration":52.247,"entry":[true,false,true,true],"geometry_index":4,"in":1,"location":[24.764716,59.442617],"out":3,"turn_duration":1.0,"turn_weight":1.0,"weight":52.247},{"admin_index":0,"bearings":[49,102,191,284],"entry":[true,false,true,true],"geometry_index":5,"in":1,"location":[24.763568,59.442739],"out":3,"turn_duration":1.0,"turn_weight":1.0}],"maneuver":{"bearing_after":254,"bearing_before":0,"instruction":"Walk west on the walkway.","location":[24.765368,59.442643],"type":"depart"},"mode":"walking","name":"","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[{"announcement":"Walk west on the walkway.","distanceAlongGeometry":111.251,"ssmlAnnouncement":"<speak>Walk west on the walkway.</speak>"},{"announcement":"In 200 feet, Turn left onto the walkway.","distanceAlongGeometry":60.0,"ssmlAnnouncement":"<speak>In 200 feet, Turn left onto the walkway.</speak>"}],"weight":92.161},{"bannerInstructions":[{"distanceAlongGeometry":9.0,"primary":{"components":[{"text":"Laeva","type":"text"}],"modifier":"right","text":"Laeva","type":"turn"}}],"distance":9.0,"driving_side":"right","duration":6.353,"geometry":"ccbkpBqbmfn@dDr@","intersections":[{"admin_index":0,"bearings":[14,104,189],"entry":[true,false,true],"geometry_index":6,"in":1,"location":[24.763449,59.442754],"out":2}],"maneuver":{"bearing_after":189,"bearing_before":284,"instruction":"Turn left onto the walkway.","location":[24.763449,59.442754],"modifier":"left","type":"turn"},"mode":"walking","name":"","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[{"announcement":"In 14 feet, Turn right onto Laeva.","distanceAlongGeometry":4.5,"ssmlAnnouncement":"<speak>In 14 feet, Turn right onto Laeva.</speak>"}],"weight":6.353},{"bannerInstructions":[{"distanceAlongGeometry":16.0,"primary":{"components":[{"text":"Bear right.","type":"text"}],"modifier":"slight right","text":"Bear right.","type":"turn"}}],"distance":16.0,"driving_side":"right","duration":12.424,"geometry":"}}akpB}`mfn@kAvO","intersections":[{"admin_index":0,"bearings":[9,101,200,286],"entry":[false,true,true,true],"geometry_index":7,"in":0,"location":[24.763423,59.442671],"out":3,"turn_weight":5.0}],"maneuver":{"bearing_after":286,"bearing_before":189,"instruction":"Turn right onto Laeva.","location":[24.763423,59.442671],"modifier":"right","type":"turn"},"mode":"walking","name":"Laeva","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[{"announcement":"In 26 feet, Bear right.","distanceAlongGeometry":8.0,"ssmlAnnouncement":"<speak>In 26 feet, Bear right.</speak>"}],"weight":17.424},{"bannerInstructions":[{"distanceAlongGeometry":15.0,"primary":{"components":[{"text":"Bear left onto the walkway.","type":"text"}],"modifier":"slight left","text":"Bear left onto the walkway.","type":"turn"}}],"distance":15.0,"driving_side":"right","duration":11.224,"geometry":"i`bkpBeplfn@oAvDkC|B","intersections":[{"admin_index":0,"bearings":[106,191,324],"entry":[false,true,true],"geometry_index":8,"in":0,"location":[24.763155,59.442709],"out":2,"turn_weight":5.0}],"maneuver":{"bearing_after":324,"bearing_before":286,"instruction":"Bear right.","location":[24.763155,59.442709],"modifier":"slight right","type":"turn"},"mode":"walking","name":"","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[{"announcement":"In 24 feet, Bear left onto the walkway.","distanceAlongGeometry":7.5,"ssmlAnnouncement":"<speak>In 24 feet, Bear left onto the walkway.</speak>"}],"weight":16.224},{"bannerInstructions":[{"distanceAlongGeometry":38.0,"primary":{"components":[{"text":"Continue.","type":"text"}],"modifier":"straight","text":"Continue.","type":"new name"}}],"distance":38.0,"driving_side":"right","duration":26.824,"geometry":"egbkpBoflfn@]~DMzAyCj^","intersections":[{"admin_index":0,"bearings":[1,70,145,287],"duration":5.647,"entry":[true,true,false,true],"geometry_index":10,"in":2,"location":[24.763,59.442819],"out":3,"weight":5.647},{"admin_index":0,"bearings":[107,158,287],"entry":[false,true,true],"geometry_index":12,"in":0,"location":[24.762858,59.442841],"out":2}],"maneuver":{"bearing_after":287,"bearing_before":325,"instruction":"Bear left onto the walkway.","location":[24.763,59.442819],"modifier":"slight left","type":"turn"},"mode":"walking","name":"","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[{"announcement":"In 62 feet, Continue.","distanceAlongGeometry":19.0,"ssmlAnnouncement":"<speak>In 62 feet, Continue.</speak>"}],"weight":26.824},{"bannerInstructions":[{"distanceAlongGeometry":7.0,"primary":{"components":[{"text":"Admiralisild; Admiral Bridge","type":"text"}],"modifier":"right","text":"Admiralisild; Admiral Bridge","type":"turn"}}],"distance":7.0,"driving_side":"right","duration":4.941,"geometry":"kmbkpBg~jfn@c@lF","intersections":[{"admin_index":0,"bearings":[107,155,287],"entry":[false,false,true],"geometry_index":13,"in":0,"location":[24.762356,59.442918],"out":2}],"maneuver":{"bearing_after":287,"bearing_before":287,"instruction":"Continue.","location":[24.762356,59.442918],"modifier":"straight","type":"new name"},"mode":"walking","name":"","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[{"announcement":"In 11 feet, Turn right onto Admiralisild/Admiral Bridge.","distanceAlongGeometry":3.5,"ssmlAnnouncement":"<speak>In 11 feet, Turn right onto Admiralisild/Admiral Bridge.</speak>"}],"weight":4.941},{"bannerInstructions":[{"distanceAlongGeometry":70.0,"primary":{"components":[{"text":"Continue on the walkway.","type":"text"}],"modifier":"straight","text":"Continue on the walkway.","type":"new name"}}],"distance":70.0,"driving_side":"right","duration":52.275,"geometry":"onbkpByvjfn@i@d@wIbHu@f@cV|P","intersections":[{"admin_index":0,"bearings":[107,168,336],"duration":16.235,"entry":[false,true,true],"geometry_index":14,"in":0,"location":[24.762237,59.442936],"out":2,"weight":16.235},{"admin_index":0,"bearings":[62,157,186,249,339],"duration":3.118,"entry":[true,false,true,true,true],"geometry_index":16,"in":1,"location":[24.762072,59.443129],"out":4,"turn_duration":1.0,"turn_weight":1.0,"weight":3.118},{"admin_index":0,"bearings":[159,338],"entry":[false,true],"geometry_index":17,"in":0,"location":[24.762052,59.443156],"out":1,"turn_weight":5.0}],"maneuver":{"bearing_after":336,"bearing_before":287,"instruction":"Turn right onto Admiralisild/Admiral Bridge.","location":[24.762237,59.442936],"modifier":"right","type":"turn"},"mode":"walking","name":"Admiralisild; Admiral Bridge","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[{"announcement":"In 200 feet, Continue on the walkway.","distanceAlongGeometry":60.0,"ssmlAnnouncement":"<speak>In 200 feet, Continue on the walkway.</speak>"}],"weight":57.275},{"bannerInstructions":[{"distanceAlongGeometry":46.0,"primary":{"components":[{"text":"Turn left onto the walkway.","type":"text"}],"modifier":"left","text":"Turn left onto the walkway.","type":"turn"}}],"distance":46.0,"driving_side":"right","duration":33.471,"geometry":"ksckpBiyifn@kA~@_TxQ","intersections":[{"admin_index":0,"bearings":[158,337],"duration":3.529,"entry":[false,true],"geometry_index":18,"in":0,"location":[24.761765,59.443526],"out":1,"turn_weight":5.0,"weight":8.529},{"admin_index":0,"bearings":[70,157,246,336],"entry":[true,false,true,true],"geometry_index":19,"in":1,"location":[24.761733,59.443564],"out":3,"turn_duration":1.0,"turn_weight":1.0}],"maneuver":{"bearing_after":337,"bearing_before":338,"instruction":"Continue on the walkway.","location":[24.761765,59.443526],"modifier":"straight","type":"new name"},"mode":"walking","name":"","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[{"announcement":"In 75 feet, Turn left onto the walkway.","distanceAlongGeometry":23.0,"ssmlAnnouncement":"<speak>In 75 feet, Turn left onto the walkway.</speak>"}],"weight":38.471},{"bannerInstructions":[{"distanceAlongGeometry":131.0,"primary":{"components":[{"text":"Turn right onto the walkway.","type":"text"}],"modifier":"right","text":"Turn right onto the walkway.","type":"turn"}}],"distance":131.0,"driving_side":"right","duration":101.718,"geometry":"wjdkpBodifn@xX|eC","intersections":[{"admin_index":0,"bearings":[55,156,249,336],"entry":[true,false,true,true],"geometry_index":20,"in":1,"location":[24.761432,59.4439],"out":2}],"maneuver":{"bearing_after":249,"bearing_before":336,"instruction":"Turn left onto the walkway.","location":[24.761432,59.4439],"modifier":"left","type":"turn"},"mode":"walking","name":"","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[{"announcement":"In 200 feet, Turn right onto the walkway.","distanceAlongGeometry":60.0,"ssmlAnnouncement":"<speak>In 200 feet, Turn right onto the walkway.</speak>"}],"weight":101.718},{"bannerInstructions":[{"distanceAlongGeometry":25.0,"primary":{"components":[{"text":"Turn left onto the walkway.","type":"text"}],"modifier":"left","text":"Turn left onto the walkway.","type":"turn"}}],"distance":25.0,"driving_side":"right","duration":21.906,"geometry":"}pckpBq}dfn@{Az@qDrBw@b@{BnA","intersections":[{"admin_index":0,"bearings":[69,251,342],"duration":3.529,"entry":[false,true,true],"geometry_index":21,"in":0,"location":[24.759273,59.443487],"out":2,"weight":3.529},{"admin_index":0,"bearings":[70,162,258,342],"duration":9.471,"entry":[false,false,false,true],"geometry_index":22,"in":1,"location":[24.759243,59.443533],"out":3,"turn_duration":1.0,"turn_weight":1.0,"weight":10.318},{"admin_index":0,"bearings":[70,162,244,342],"entry":[true,false,true,true],"geometry_index":23,"in":1,"location":[24.759185,59.443622],"out":3,"turn_duration":1.0,"turn_weight":1.0}],"maneuver":{"bearing_after":342,"bearing_before":249,"instruction":"Turn right onto the walkway.","location":[24.759273,59.443487],"modifier":"right","type":"turn"},"mode":"walking","name":"","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[{"announcement":"In 41 feet, Turn left onto the walkway.","distanceAlongGeometry":12.5,"ssmlAnnouncement":"<speak>In 41 feet, Turn left onto the walkway.</speak>"}],"weight":23.148},{"bannerInstructions":[{"distanceAlongGeometry":16.0,"primary":{"components":[{"text":"Logi","type":"text"}],"modifier":"right","text":"Logi","type":"turn"}}],"distance":16.0,"driving_side":"right","duration":12.294,"geometry":"__dkpBmtdfn@TbCNjBd@rH","intersections":[{"admin_index":0,"bearings":[77,162,253,348],"duration":4.941,"entry":[true,false,true,true],"geometry_index":25,"in":1,"location":[24.759127,59.443712],"out":2,"weight":4.941},{"admin_index":0,"bearings":[73,168,256,335],"entry":[false,true,true,true],"geometry_index":27,"in":0,"location":[24.759007,59.443693],"out":2,"turn_duration":1.0,"turn_weight":1.0}],"maneuver":{"bearing_after":253,"bearing_before":342,"instruction":"Turn left onto the walkway.","location":[24.759127,59.443712],"modifier":"left","type":"turn"},"mode":"walking","name":"","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[{"announcement":"In 26 feet, Turn right onto Logi.","distanceAlongGeometry":8.0,"ssmlAnnouncement":"<speak>In 26 feet, Turn right onto Logi.</speak>"}],"weight":12.294},{"bannerInstructions":[{"distanceAlongGeometry":91.0,"primary":{"components":[{"text":"Turn left onto the walkway.","type":"text"}],"modifier":"left","text":"Turn left onto the walkway.","type":"turn"}}],"distance":91.0,"driving_side":"right","duration":72.235,"geometry":"s|ckpBicdfn@yAj@g@JiDrAcJxDcBjBcA^sDvAsIjDmCnD}@R","intersections":[{"admin_index":0,"bearings":[76,163,258,348],"duration":4.941,"entry":[false,true,true,true],"geometry_index":28,"in":0,"location":[24.758853,59.443674],"out":3,"weight":4.941},{"admin_index":0,"bearings":[61,168,248,345],"duration":28.118,"entry":[true,false,true,true],"geometry_index":30,"in":1,"location":[24.758825,59.443739],"out":3,"turn_duration":2.0,"turn_weight":7.0,"weight":33.118},{"admin_index":0,"bearings":[161,347],"duration":2.824,"entry":[false,true],"geometry_index":33,"in":0,"location":[24.758636,59.444052],"out":1,"weight":2.824},{"admin_index":0,"bearings":[82,167,258,346],"duration":8.059,"entry":[true,false,true,true],"geometry_index":34,"in":1,"location":[24.75862,59.444086],"out":3,"turn_duration":1.0,"turn_weight":1.0,"weight":8.059},{"admin_index":0,"bearings":[18,166,246,346],"duration":19.118,"entry":[true,false,true,true],"geometry_index":35,"in":1,"location":[24.758576,59.444176],"out":3,"turn_duration":5.0,"turn_weight":5.0,"weight":19.118},{"admin_index":0,"bearings":[166,253,328],"duration":6.353,"entry":[false,true,true],"geometry_index":36,"in":0,"location":[24.75849,59.444346],"out":2,"weight":6.353},{"admin_index":0,"bearings":[148,213,351],"entry":[false,true,true],"geometry_index":37,"in":0,"location":[24.758402,59.444417],"out":2}],"maneuver":{"bearing_after":348,"bearing_before":256,"instruction":"Turn right onto Logi.","location":[24.758853,59.443674],"modifier":"right","type":"turn"},"mode":"walking","name":"Logi","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[{"announcement":"In 200 feet, Turn left onto the walkway.","distanceAlongGeometry":60.0,"ssmlAnnouncement":"<speak>In 200 feet, Turn left onto the walkway.</speak>"}],"weight":77.235},{"bannerInstructions":[{"distanceAlongGeometry":8.0,"primary":{"components":[{"text":"Turn right onto the walkway.","type":"text"}],"modifier":"right","text":"Turn right onto the walkway.","type":"turn"}}],"distance":8.0,"driving_side":"right","duration":5.647,"geometry":"_mekpBofcfn@`@bH","intersections":[{"admin_index":0,"bearings":[77,171,257,346],"entry":[true,false,true,true],"geometry_index":38,"in":1,"location":[24.758392,59.444448],"out":2,"turn_weight":5.0}],"maneuver":{"bearing_after":257,"bearing_before":351,"instruction":"Turn left onto the walkway.","location":[24.758392,59.444448],"modifier":"left","type":"turn"},"mode":"walking","name":"","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[{"announcement":"In 13 feet, Turn right onto the walkway.","distanceAlongGeometry":4.0,"ssmlAnnouncement":"<speak>In 13 feet, Turn right onto the walkway.</speak>"}],"weight":10.647},{"bannerInstructions":[{"distanceAlongGeometry":85.0,"primary":{"components":[{"text":"Kultuurikilomeeter","type":"text"}],"modifier":"slight left","text":"Kultuurikilomeeter","type":"turn"}}],"distance":85.0,"driving_side":"right","duration":64.447,"geometry":"}kekpBk}bfn@gHnBsRvGkDhCsDpT","intersections":[{"admin_index":0,"bearings":[77,213,349],"duration":48.918,"entry":[false,true,true],"geometry_index":39,"in":0,"location":[24.758246,59.444431],"out":2,"weight":48.918},{"admin_index":0,"bearings":[75,158,297],"entry":[true,false,true],"geometry_index":42,"in":1,"location":[24.757981,59.444979],"out":2}],"maneuver":{"bearing_after":349,"bearing_before":257,"instruction":"Turn right onto the walkway.","location":[24.758246,59.444431],"modifier":"right","type":"turn"},"mode":"walking","name":"","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[{"announcement":"In 200 feet, Bear left onto Kultuurikilomeeter.","distanceAlongGeometry":60.0,"ssmlAnnouncement":"<speak>In 200 feet, Bear left onto Kultuurikilomeeter.</speak>"}],"weight":64.447},{"bannerInstructions":[{"distanceAlongGeometry":1254.0,"primary":{"components":[{"text":"Turn right onto the walkway.","type":"text"}],"modifier":"right","text":"Turn right onto the walkway.","type":"turn"}}],"distance":1254.0,"driving_side":"right","duration":966.424,"geometry":"ysfkpBgwafn@pF~dEPfMfAft@H~FNrEdAt}@f@pY@rA?`@@rBJhRCdAIbD]nFa@bDaAbIiAdImB~MKt@wGrd@qBnOoDbUwAxJVfH\\jMHpEGzAiAjDqMbf@gBnFkC~HeDbKs@vBkCtF}CpGuIzNU`@oGzH{FhGqi@hc@ud@t_@wIpI{JfNqLfTwJjVgDdJ_HvYaEpUgHxa@aFhd@mErt@q@~FmFrd@oJdw@","intersections":[{"admin_index":0,"bearings":[117,266,329],"duration":127.059,"entry":[false,true,true],"geometry_index":43,"in":0,"location":[24.757636,59.445069],"out":1,"turn_weight":5.0,"weight":132.059},{"admin_index":0,"bearings":[86,175,266,355],"duration":13.205,"entry":[false,true,true,true],"geometry_index":44,"in":0,"location":[24.754468,59.444948],"out":2,"turn_duration":1.0,"turn_weight":1.0,"weight":15.035},{"admin_index":0,"bearings":[86,265],"duration":39.529,"entry":[false,true],"geometry_index":45,"in":0,"location":[24.75424,59.444939],"out":1,"weight":39.529},{"admin_index":0,"bearings":[86,262],"duration":4.235,"entry":[false,true],"geometry_index":47,"in":0,"location":[24.75326,59.444898],"out":1,"weight":4.235},{"admin_index":0,"bearings":[82,176,266],"duration":62.104,"entry":[false,true,true],"geometry_index":48,"in":0,"location":[24.753154,59.44489],"out":2,"weight":62.104},{"admin_index":0,"bearings":[86,176,268,358],"duration":3.824,"entry":[false,true,true,true],"geometry_index":51,"in":0,"location":[24.751684,59.444834],"out":2,"turn_duration":1.0,"turn_weight":6.0,"weight":8.824},{"admin_index":0,"bearings":[88,176,268,358],"duration":37.339,"entry":[false,true,true,true],"geometry_index":53,"in":0,"location":[24.751609,59.444833],"out":2,"turn_duration":1.0,"turn_weight":1.0,"weight":44.607},{"admin_index":0,"bearings":[109,134,292],"duration":15.529,"entry":[false,true,true],"geometry_index":58,"in":0,"location":[24.750981,59.444866],"out":2,"weight":15.529},{"admin_index":0,"bearings":[114,156,294],"duration":10.588,"entry":[false,true,true],"geometry_index":60,"in":0,"location":[24.750656,59.444936],"out":2,"weight":10.588},{"admin_index":0,"bearings":[114,169,294,359],"duration":39.824,"entry":[false,true,true,true],"geometry_index":61,"in":0,"location":[24.750416,59.444991],"out":2,"turn_duration":1.0,"turn_weight":1.0,"weight":39.824},{"admin_index":0,"bearings":[34,113,296],"duration":15.529,"entry":[true,false,true],"geometry_index":64,"in":1,"location":[24.749523,59.445194],"out":2,"weight":15.529},{"admin_index":0,"bearings":[116,295],"duration":14.118,"entry":[false,true],"geometry_index":65,"in":0,"location":[24.749169,59.445282],"out":1,"weight":14.118},{"admin_index":0,"bearings":[81,263,345],"duration":13.122,"entry":[false,true,true],"geometry_index":67,"in":0,"location":[24.748832,59.445314],"out":1,"weight":15.747},{"admin_index":0,"bearings":[83,183,268,351],"duration":8.353,"entry":[false,true,true,true],"geometry_index":68,"in":0,"location":[24.748602,59.445299],"out":2,"turn_duration":2.0,"turn_weight":7.0,"weight":13.353},{"admin_index":0,"bearings":[90,194,310],"duration":53.125,"entry":[false,true,true],"geometry_index":70,"in":0,"location":[24.748451,59.445298],"out":2,"weight":53.125},{"admin_index":0,"bearings":[131,222,310],"duration":21.699,"entry":[false,true,true],"geometry_index":74,"in":0,"location":[24.747459,59.44569],"out":2,"weight":21.699},{"admin_index":0,"bearings":[45,138,319],"duration":33.798,"entry":[true,false,true],"geometry_index":77,"in":1,"location":[24.747082,59.445869],"out":2,"weight":38.867},{"admin_index":0,"bearings":[56,143,237,328],"duration":100.953,"entry":[true,false,true,true],"geometry_index":79,"in":1,"location":[24.746691,59.446119],"out":3,"turn_duration":1.0,"turn_weight":1.0,"weight":105.951},{"admin_index":0,"bearings":[65,157,249,336],"duration":68.059,"entry":[true,false,true,true],"geometry_index":83,"in":1,"location":[24.745802,59.447073],"out":3,"turn_duration":1.0,"turn_weight":1.0,"weight":68.059},{"admin_index":0,"bearings":[61,153,244,327],"duration":80.059,"entry":[true,false,true,true],"geometry_index":85,"in":1,"location":[24.74511,59.447848],"out":3,"turn_duration":1.0,"turn_weight":1.0,"weight":84.012},{"admin_index":0,"bearings":[38,133,304],"duration":32.139,"entry":[true,false,true],"geometry_index":89,"in":1,"location":[24.743973,59.448527],"out":2,"weight":40.174},{"admin_index":0,"bearings":[124,215,298],"duration":102.353,"entry":[false,true,true],"geometry_index":90,"in":0,"location":[24.743545,59.448671],"out":2,"weight":102.353},{"admin_index":0,"bearings":[103,121,291],"duration":5.647,"entry":[false,true,true],"geometry_index":94,"in":0,"location":[24.741172,59.449132],"out":2,"weight":5.647},{"admin_index":0,"bearings":[20,111,291],"entry":[true,false,true],"geometry_index":95,"in":1,"location":[24.741044,59.449157],"out":2}],"maneuver":{"bearing_after":266,"bearing_before":297,"instruction":"Bear left onto Kultuurikilomeeter.","location":[24.757636,59.445069],"modifier":"slight left","type":"turn"},"mode":"walking","name":"Kultuurikilomeeter","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[{"announcement":"In 200 feet, Turn right onto the walkway.","distanceAlongGeometry":60.0,"ssmlAnnouncement":"<speak>In 200 feet, Turn right onto the walkway.</speak>"}],"weight":1015.202},{"bannerInstructions":[{"distanceAlongGeometry":23.0,"primary":{"components":[{"text":"Turn left onto the walkway.","type":"text"}],"modifier":"left","text":"Turn left onto the walkway.","type":"turn"}}],"distance":23.0,"driving_side":"right","duration":18.235,"geometry":"gfokpBml~dn@kFmDsCyA","intersections":[{"admin_index":0,"bearings":[21,112,291],"duration":9.882,"entry":[true,false,true],"geometry_index":97,"in":1,"location":[24.739543,59.44946],"out":0,"turn_weight":5.0,"weight":14.882},{"admin_index":0,"bearings":[17,115,201,291],"entry":[true,true,false,true],"geometry_index":98,"in":2,"location":[24.73963,59.449578],"out":0,"turn_duration":2.0,"turn_weight":2.0}],"maneuver":{"bearing_after":21,"bearing_before":292,"instruction":"Turn right onto the walkway.","location":[24.739543,59.44946],"modifier":"right","type":"turn"},"mode":"walking","name":"","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[{"announcement":"In 37 feet, Turn left onto the walkway.","distanceAlongGeometry":11.5,"ssmlAnnouncement":"<speak>In 37 feet, Turn left onto the walkway.</speak>"}],"weight":23.235},{"bannerInstructions":[{"distanceAlongGeometry":16.0,"primary":{"components":[{"text":"Turn left onto the crosswalk.","type":"text"}],"modifier":"left","text":"Turn left onto the crosswalk.","type":"turn"}}],"distance":16.0,"driving_side":"right","duration":11.294,"geometry":"grokpBut~dn@yArJgAdA","intersections":[{"admin_index":0,"bearings":[111,197,304],"entry":[true,false,true],"geometry_index":99,"in":1,"location":[24.739675,59.449652],"out":2}],"maneuver":{"bearing_after":304,"bearing_before":17,"instruction":"Turn left onto the walkway.","location":[24.739675,59.449652],"modifier":"left","type":"turn"},"mode":"walking","name":"","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[{"announcement":"In 26 feet, Turn left onto the crosswalk.","distanceAlongGeometry":8.0,"ssmlAnnouncement":"<speak>In 26 feet, Turn left onto the crosswalk.</speak>"}],"weight":11.294},{"bannerInstructions":[{"distanceAlongGeometry":347.0,"primary":{"components":[{"text":"Turn right onto the walkway.","type":"text"}],"modifier":"right","text":"Turn right onto the walkway.","type":"turn"}}],"distance":347.0,"driving_side":"right","duration":263.849,"geometry":"iwokpB{f~dn@JhDm@`G_@fCMrAmAfFiKf|@{Fxh@oCdSi@dGaBrQcBbNwCd\\kGlh@uA~PuEzr@_@bHa@dC}@\\KbEOvC","intersections":[{"admin_index":0,"bearings":[8,127,262],"duration":3.529,"entry":[true,false,true],"geometry_index":101,"in":1,"location":[24.739454,59.449733],"out":2,"weight":3.529},{"admin_index":0,"bearings":[82,156,289,358],"duration":6.647,"entry":[false,true,true,true],"geometry_index":102,"in":0,"location":[24.739369,59.449727],"out":2,"turn_duration":1.0,"turn_weight":1.0,"weight":6.647},{"admin_index":0,"bearings":[44,109,255,295],"duration":3.823,"entry":[true,false,true,true],"geometry_index":103,"in":1,"location":[24.73924,59.44975],"out":3,"turn_duration":1.0,"turn_weight":1.0,"weight":3.823},{"admin_index":0,"bearings":[15,115,297],"duration":82.306,"entry":[true,false,true],"geometry_index":104,"in":1,"location":[24.739172,59.449766],"out":2,"weight":82.306},{"admin_index":0,"bearings":[110,201,294],"duration":15.529,"entry":[false,true,true],"geometry_index":108,"in":0,"location":[24.737365,59.450135],"out":2,"weight":15.529},{"admin_index":0,"bearings":[20,114,207,288],"duration":6.647,"entry":[true,false,true,true],"geometry_index":109,"in":1,"location":[24.737042,59.450207],"out":3,"turn_duration":1.0,"turn_weight":1.0,"weight":6.647},{"admin_index":0,"bearings":[32,108,288],"duration":42.353,"entry":[true,false,true],"geometry_index":110,"in":1,"location":[24.736911,59.450228],"out":2,"weight":42.353},{"admin_index":0,"bearings":[34,108,292],"duration":82.306,"entry":[true,false,true],"geometry_index":113,"in":1,"location":[24.735904,59.450403],"out":2,"weight":82.306},{"admin_index":0,"bearings":[104,191,295],"duration":12.649,"entry":[false,true,true],"geometry_index":116,"in":0,"location":[24.734123,59.450687],"out":2,"weight":13.282},{"admin_index":0,"bearings":[13,120,277],"duration":4.235,"entry":[true,false,true],"geometry_index":119,"in":1,"location":[24.733895,59.450751],"out":2,"weight":4.235},{"admin_index":0,"bearings":[14,97,193,282],"entry":[true,false,true,true],"geometry_index":120,"in":1,"location":[24.733797,59.450757],"out":3,"turn_duration":1.0,"turn_weight":1.0}],"maneuver":{"bearing_after":262,"bearing_before":307,"instruction":"Turn left onto the crosswalk.","location":[24.739454,59.449733],"modifier":"left","type":"turn"},"mode":"walking","name":"","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[{"announcement":"In 200 feet, Turn right onto the walkway.","distanceAlongGeometry":60.0,"ssmlAnnouncement":"<speak>In 200 feet, Turn right onto the walkway.</speak>"}],"weight":264.482},{"bannerInstructions":[{"distanceAlongGeometry":2.0,"primary":{"components":[{"text":"Turn left onto the walkway.","type":"text"}],"modifier":"left","text":"Turn left onto the walkway.","type":"turn"}}],"distance":2.0,"driving_side":"right","duration":1.412,"geometry":"ywqkpBq`sdn@k@F","intersections":[{"admin_index":0,"bearings":[102,269,355],"entry":[false,true,true],"geometry_index":121,"in":0,"location":[24.733721,59.450765],"out":2}],"maneuver":{"bearing_after":355,"bearing_before":282,"instruction":"Turn right onto the walkway.","location":[24.733721,59.450765],"modifier":"right","type":"turn"},"mode":"walking","name":"","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[{"announcement":"In 3 feet, Turn left onto the walkway.","distanceAlongGeometry":1.0,"ssmlAnnouncement":"<speak>In 3 feet, Turn left onto the walkway.</speak>"}],"weight":1.412},{"bannerInstructions":[{"distanceAlongGeometry":241.0,"primary":{"components":[{"text":"Allveelaeva","type":"text"}],"modifier":"slight left","text":"Allveelaeva","type":"turn"}}],"distance":241.0,"driving_side":"right","duration":184.456,"geometry":"eyqkpBi`sdn@oQbw@uNno@Gv@SxCo@hEiA`@i@nBf@pCQtDk@xC{B|KgTraAuA\\i@o@","intersections":[{"admin_index":0,"bearings":[14,175,303],"duration":45.642,"entry":[true,false,true],"geometry_index":122,"in":1,"location":[24.733717,59.450787],"out":2,"weight":45.642},{"admin_index":0,"bearings":[100,123,302],"duration":41.929,"entry":[true,false,true],"geometry_index":123,"in":1,"location":[24.732819,59.451083],"out":2,"weight":41.929},{"admin_index":0,"bearings":[119,213,284],"duration":2.823,"entry":[false,true,true],"geometry_index":125,"in":0,"location":[24.732015,59.451338],"out":2,"weight":2.823},{"admin_index":0,"bearings":[31,104,214,303],"duration":19.635,"entry":[true,false,true,true],"geometry_index":126,"in":1,"location":[24.731938,59.451348],"out":3,"turn_duration":1.0,"turn_weight":1.0,"weight":19.635},{"admin_index":0,"bearings":[25,89,193,299],"duration":4.529,"entry":[true,false,true,true],"geometry_index":131,"in":1,"location":[24.7316,59.451419],"out":3,"turn_duration":1.0,"turn_weight":1.0,"weight":4.529},{"admin_index":0,"bearings":[119,194,301],"duration":14.132,"entry":[false,true,true],"geometry_index":132,"in":0,"location":[24.731523,59.451441],"out":2,"weight":16.958},{"admin_index":0,"bearings":[38,121,302],"entry":[true,false,true],"geometry_index":133,"in":1,"location":[24.731316,59.451503],"out":2}],"maneuver":{"bearing_after":303,"bearing_before":355,"instruction":"Turn left onto the walkway.","location":[24.733717,59.450787],"modifier":"left","type":"turn"},"mode":"walking","name":"","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[{"announcement":"In 200 feet, Bear left onto Allveelaeva.","distanceAlongGeometry":60.0,"ssmlAnnouncement":"<speak>In 200 feet, Bear left onto Allveelaeva.</speak>"}],"weight":187.283},{"bannerInstructions":[{"distanceAlongGeometry":28.0,"primary":{"components":[{"text":"Peetri","type":"text"}],"modifier":"right","text":"Peetri","type":"turn"}}],"distance":28.0,"driving_side":"right","duration":20.951,"geometry":"e_tkpBehldn@mFzY","intersections":[{"admin_index":0,"bearings":[32,120,152,299],"entry":[true,true,false,true],"geometry_index":136,"in":2,"location":[24.730259,59.451907],"out":3,"turn_weight":5.0}],"maneuver":{"bearing_after":299,"bearing_before":332,"instruction":"Bear left onto Allveelaeva.","location":[24.730259,59.451907],"modifier":"slight left","type":"turn"},"mode":"walking","name":"Allveelaeva","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[{"announcement":"In 45 feet, Turn right onto Peetri.","distanceAlongGeometry":14.0,"ssmlAnnouncement":"<speak>In 45 feet, Turn right onto Peetri.</speak>"}],"weight":25.951},{"bannerInstructions":[{"distanceAlongGeometry":25.099,"primary":{"components":[{"text":"You have arrived at your destination.","type":"text"}],"text":"You have arrived at your destination.","type":"arrive"}}],"distance":25.099,"driving_side":"right","duration":24.804,"geometry":"sftkpBimkdn@}GiGqBoC","intersections":[{"admin_index":0,"bearings":[1,25,119,208],"entry":[true,true,false,true],"geometry_index":137,"in":2,"location":[24.729829,59.452026],"out":1,"turn_weight":5.0}],"maneuver":{"bearing_after":25,"bearing_before":299,"instruction":"Turn right onto Peetri.","location":[24.729829,59.452026],"modifier":"right","type":"turn"},"mode":"walking","name":"Peetri","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[{"announcement":"In 41 feet, You have arrived at your destination.","distanceAlongGeometry":12.5495,"ssmlAnnouncement":"<speak>In 41 feet, You have arrived at your destination.</speak>"}],"weight":54.607},{"bannerInstructions":[{"distanceAlongGeometry":0.0,"primary":{"components":[{"text":"You have arrived at your destination.","type":"text"}],"text":"You have arrived at your destination.","type":"arrive"}}],"distance":0.0,"driving_side":"right","duration":0.0,"geometry":"cstkpBczkdn@??","intersections":[{"admin_index":0,"bearings":[213],"entry":[true],"geometry_index":139,"in":0,"location":[24.730034,59.452226]}],"maneuver":{"bearing_after":0,"bearing_before":33,"instruction":"You have arrived at your destination.","location":[24.730034,59.452226],"type":"arrive"},"mode":"walking","name":"Peetri","speedLimitSign":"vienna","speedLimitUnit":"km/h","voiceInstructions":[],"weight":0.0}],"summary":"Logi, Kultuurikilomeeter","via_waypoints":[],"weight":2132.626}],"weight":2132.626,"weight_name":"pedestrian"}],"waypoints":[{"distance":0.546,"location":[24.765368,59.442643],"name":""},{"distance":0.134,"location":[24.730034,59.452226],"name":"Peetri"}]}"#;

    #[test]
    fn parse_standard_osrm() {
        let parser = OsrmResponseParser::new(6);
        let response = parser
            .parse_response(STANDARD_OSRM_POLYLINE6_RESPONSE.into())
            .expect("Unable to parse OSRM response");
        assert_eq!(response.len(), 1);

        // Verify the geometry
        let expected_coords = vec![
            GeographicCoordinates {
                lat: 52.517033,
                lng: 13.388798,
            },
            GeographicCoordinates {
                lat: 52.527168,
                lng: 13.387228,
            },
            GeographicCoordinates {
                lat: 52.528491,
                lng: 13.393668,
            },
            GeographicCoordinates {
                lat: 52.529432,
                lng: 13.39763,
            },
            GeographicCoordinates {
                lat: 52.529684,
                lng: 13.403888,
            },
            GeographicCoordinates {
                lat: 52.528326,
                lng: 13.411389,
            },
            GeographicCoordinates {
                lat: 52.527507,
                lng: 13.41432,
            },
            GeographicCoordinates {
                lat: 52.52677,
                lng: 13.415657,
            },
            GeographicCoordinates {
                lat: 52.528458,
                lng: 13.417166,
            },
            GeographicCoordinates {
                lat: 52.528728,
                lng: 13.421348,
            },
            GeographicCoordinates {
                lat: 52.528082,
                lng: 13.424085,
            },
            GeographicCoordinates {
                lat: 52.528068,
                lng: 13.424993,
            },
            GeographicCoordinates {
                lat: 52.527885,
                lng: 13.425184,
            },
            GeographicCoordinates {
                lat: 52.527043,
                lng: 13.427263,
            },
            GeographicCoordinates {
                lat: 52.525063,
                lng: 13.43036,
            },
            GeographicCoordinates {
                lat: 52.52479,
                lng: 13.430413,
            },
            GeographicCoordinates {
                lat: 52.523269,
                lng: 13.429678,
            },
            GeographicCoordinates {
                lat: 52.523239,
                lng: 13.428554,
            },
        ];
        assert_eq!(response[0].geometry, expected_coords);
    }

    #[test]
    fn parse_valhalla_osrm() {
        let parser = OsrmResponseParser::new(6);
        let response = parser
            .parse_response(VALHALLA_OSRM_RESPONSE.into())
            .expect("Unable to parse Valhalla OSRM response");
        assert_eq!(response.len(), 1);

        // Verify the geometry
        let expected_coords = vec![
            GeographicCoordinates {
                lng: 24.765368,
                lat: 59.442643,
            },
            GeographicCoordinates {
                lng: 24.765372,
                lat: 59.442644,
            },
            GeographicCoordinates {
                lng: 24.765043,
                lat: 59.442596,
            },
            GeographicCoordinates {
                lng: 24.764917,
                lat: 59.442597,
            },
            GeographicCoordinates {
                lng: 24.764716,
                lat: 59.442617,
            },
            GeographicCoordinates {
                lng: 24.763568,
                lat: 59.442739,
            },
            GeographicCoordinates {
                lng: 24.763449,
                lat: 59.442754,
            },
            GeographicCoordinates {
                lng: 24.763423,
                lat: 59.442671,
            },
            GeographicCoordinates {
                lng: 24.763155,
                lat: 59.442709,
            },
            GeographicCoordinates {
                lng: 24.763063,
                lat: 59.442749,
            },
            GeographicCoordinates {
                lng: 24.763,
                lat: 59.442819,
            },
            GeographicCoordinates {
                lng: 24.762904,
                lat: 59.442834,
            },
            GeographicCoordinates {
                lng: 24.762858,
                lat: 59.442841,
            },
            GeographicCoordinates {
                lng: 24.762356,
                lat: 59.442918,
            },
            GeographicCoordinates {
                lng: 24.762237,
                lat: 59.442936,
            },
            GeographicCoordinates {
                lng: 24.762218,
                lat: 59.442957,
            },
            GeographicCoordinates {
                lng: 24.762072,
                lat: 59.443129,
            },
            GeographicCoordinates {
                lng: 24.762052,
                lat: 59.443156,
            },
            GeographicCoordinates {
                lng: 24.761765,
                lat: 59.443526,
            },
            GeographicCoordinates {
                lng: 24.761733,
                lat: 59.443564,
            },
            GeographicCoordinates {
                lng: 24.761432,
                lat: 59.4439,
            },
            GeographicCoordinates {
                lng: 24.759273,
                lat: 59.443487,
            },
            GeographicCoordinates {
                lng: 24.759243,
                lat: 59.443533,
            },
            GeographicCoordinates {
                lng: 24.759185,
                lat: 59.443622,
            },
            GeographicCoordinates {
                lng: 24.759167,
                lat: 59.44365,
            },
            GeographicCoordinates {
                lng: 24.759127,
                lat: 59.443712,
            },
            GeographicCoordinates {
                lng: 24.759061,
                lat: 59.443701,
            },
            GeographicCoordinates {
                lng: 24.759007,
                lat: 59.443693,
            },
            GeographicCoordinates {
                lng: 24.758853,
                lat: 59.443674,
            },
            GeographicCoordinates {
                lng: 24.758831,
                lat: 59.443719,
            },
            GeographicCoordinates {
                lng: 24.758825,
                lat: 59.443739,
            },
            GeographicCoordinates {
                lng: 24.758783,
                lat: 59.443824,
            },
            GeographicCoordinates {
                lng: 24.75869,
                lat: 59.444002,
            },
            GeographicCoordinates {
                lng: 24.758636,
                lat: 59.444052,
            },
            GeographicCoordinates {
                lng: 24.75862,
                lat: 59.444086,
            },
            GeographicCoordinates {
                lng: 24.758576,
                lat: 59.444176,
            },
            GeographicCoordinates {
                lng: 24.75849,
                lat: 59.444346,
            },
            GeographicCoordinates {
                lng: 24.758402,
                lat: 59.444417,
            },
            GeographicCoordinates {
                lng: 24.758392,
                lat: 59.444448,
            },
            GeographicCoordinates {
                lng: 24.758246,
                lat: 59.444431,
            },
            GeographicCoordinates {
                lng: 24.75819,
                lat: 59.444579,
            },
            GeographicCoordinates {
                lng: 24.75805,
                lat: 59.444893,
            },
            GeographicCoordinates {
                lng: 24.757981,
                lat: 59.444979,
            },
            GeographicCoordinates {
                lng: 24.757636,
                lat: 59.445069,
            },
            GeographicCoordinates {
                lng: 24.754468,
                lat: 59.444948,
            },
            GeographicCoordinates {
                lng: 24.75424,
                lat: 59.444939,
            },
            GeographicCoordinates {
                lng: 24.753388,
                lat: 59.444903,
            },
            GeographicCoordinates {
                lng: 24.75326,
                lat: 59.444898,
            },
            GeographicCoordinates {
                lng: 24.753154,
                lat: 59.44489,
            },
            GeographicCoordinates {
                lng: 24.752151,
                lat: 59.444855,
            },
            GeographicCoordinates {
                lng: 24.751726,
                lat: 59.444835,
            },
            GeographicCoordinates {
                lng: 24.751684,
                lat: 59.444834,
            },
            GeographicCoordinates {
                lng: 24.751667,
                lat: 59.444834,
            },
            GeographicCoordinates {
                lng: 24.751609,
                lat: 59.444833,
            },
            GeographicCoordinates {
                lng: 24.7513,
                lat: 59.444827,
            },
            GeographicCoordinates {
                lng: 24.751265,
                lat: 59.444829,
            },
            GeographicCoordinates {
                lng: 24.751183,
                lat: 59.444834,
            },
            GeographicCoordinates {
                lng: 24.751063,
                lat: 59.444849,
            },
            GeographicCoordinates {
                lng: 24.750981,
                lat: 59.444866,
            },
            GeographicCoordinates {
                lng: 24.750819,
                lat: 59.444899,
            },
            GeographicCoordinates {
                lng: 24.750656,
                lat: 59.444936,
            },
            GeographicCoordinates {
                lng: 24.750416,
                lat: 59.444991,
            },
            GeographicCoordinates {
                lng: 24.750389,
                lat: 59.444997,
            },
            GeographicCoordinates {
                lng: 24.749787,
                lat: 59.445137,
            },
            GeographicCoordinates {
                lng: 24.749523,
                lat: 59.445194,
            },
            GeographicCoordinates {
                lng: 24.749169,
                lat: 59.445282,
            },
            GeographicCoordinates {
                lng: 24.74898,
                lat: 59.445326,
            },
            GeographicCoordinates {
                lng: 24.748832,
                lat: 59.445314,
            },
            GeographicCoordinates {
                lng: 24.748602,
                lat: 59.445299,
            },
            GeographicCoordinates {
                lng: 24.748497,
                lat: 59.445294,
            },
            GeographicCoordinates {
                lng: 24.748451,
                lat: 59.445298,
            },
            GeographicCoordinates {
                lng: 24.748365,
                lat: 59.445335,
            },
            GeographicCoordinates {
                lng: 24.747739,
                lat: 59.445568,
            },
            GeographicCoordinates {
                lng: 24.747619,
                lat: 59.44562,
            },
            GeographicCoordinates {
                lng: 24.747459,
                lat: 59.44569,
            },
            GeographicCoordinates {
                lng: 24.747265,
                lat: 59.445773,
            },
            GeographicCoordinates {
                lng: 24.747205,
                lat: 59.445799,
            },
            GeographicCoordinates {
                lng: 24.747082,
                lat: 59.445869,
            },
            GeographicCoordinates {
                lng: 24.746945,
                lat: 59.445948,
            },
            GeographicCoordinates {
                lng: 24.746691,
                lat: 59.446119,
            },
            GeographicCoordinates {
                lng: 24.746674,
                lat: 59.44613,
            },
            GeographicCoordinates {
                lng: 24.746516,
                lat: 59.446266,
            },
            GeographicCoordinates {
                lng: 24.746383,
                lat: 59.446392,
            },
            GeographicCoordinates {
                lng: 24.745802,
                lat: 59.447073,
            },
            GeographicCoordinates {
                lng: 24.745279,
                lat: 59.447676,
            },
            GeographicCoordinates {
                lng: 24.74511,
                lat: 59.447848,
            },
            GeographicCoordinates {
                lng: 24.744866,
                lat: 59.448038,
            },
            GeographicCoordinates {
                lng: 24.744526,
                lat: 59.448255,
            },
            GeographicCoordinates {
                lng: 24.744152,
                lat: 59.448443,
            },
            GeographicCoordinates {
                lng: 24.743973,
                lat: 59.448527,
            },
            GeographicCoordinates {
                lng: 24.743545,
                lat: 59.448671,
            },
            GeographicCoordinates {
                lng: 24.743184,
                lat: 59.448768,
            },
            GeographicCoordinates {
                lng: 24.742627,
                lat: 59.448916,
            },
            GeographicCoordinates {
                lng: 24.74203,
                lat: 59.449029,
            },
            GeographicCoordinates {
                lng: 24.741172,
                lat: 59.449132,
            },
            GeographicCoordinates {
                lng: 24.741044,
                lat: 59.449157,
            },
            GeographicCoordinates {
                lng: 24.740442,
                lat: 59.449276,
            },
            GeographicCoordinates {
                lng: 24.739543,
                lat: 59.44946,
            },
            GeographicCoordinates {
                lng: 24.73963,
                lat: 59.449578,
            },
            GeographicCoordinates {
                lng: 24.739675,
                lat: 59.449652,
            },
            GeographicCoordinates {
                lng: 24.739489,
                lat: 59.449697,
            },
            GeographicCoordinates {
                lng: 24.739454,
                lat: 59.449733,
            },
            GeographicCoordinates {
                lng: 24.739369,
                lat: 59.449727,
            },
            GeographicCoordinates {
                lng: 24.73924,
                lat: 59.44975,
            },
            GeographicCoordinates {
                lng: 24.739172,
                lat: 59.449766,
            },
            GeographicCoordinates {
                lng: 24.73913,
                lat: 59.449773,
            },
            GeographicCoordinates {
                lng: 24.739014,
                lat: 59.449812,
            },
            GeographicCoordinates {
                lng: 24.738034,
                lat: 59.450009,
            },
            GeographicCoordinates {
                lng: 24.737365,
                lat: 59.450135,
            },
            GeographicCoordinates {
                lng: 24.737042,
                lat: 59.450207,
            },
            GeographicCoordinates {
                lng: 24.736911,
                lat: 59.450228,
            },
            GeographicCoordinates {
                lng: 24.736613,
                lat: 59.450277,
            },
            GeographicCoordinates {
                lng: 24.736371,
                lat: 59.450327,
            },
            GeographicCoordinates {
                lng: 24.735904,
                lat: 59.450403,
            },
            GeographicCoordinates {
                lng: 24.735241,
                lat: 59.450537,
            },
            GeographicCoordinates {
                lng: 24.734953,
                lat: 59.45058,
            },
            GeographicCoordinates {
                lng: 24.734123,
                lat: 59.450687,
            },
            GeographicCoordinates {
                lng: 24.733977,
                lat: 59.450703,
            },
            GeographicCoordinates {
                lng: 24.73391,
                lat: 59.45072,
            },
            GeographicCoordinates {
                lng: 24.733895,
                lat: 59.450751,
            },
            GeographicCoordinates {
                lng: 24.733797,
                lat: 59.450757,
            },
            GeographicCoordinates {
                lng: 24.733721,
                lat: 59.450765,
            },
            GeographicCoordinates {
                lng: 24.733717,
                lat: 59.450787,
            },
            GeographicCoordinates {
                lng: 24.732819,
                lat: 59.451083,
            },
            GeographicCoordinates {
                lng: 24.732043,
                lat: 59.451334,
            },
            GeographicCoordinates {
                lng: 24.732015,
                lat: 59.451338,
            },
            GeographicCoordinates {
                lng: 24.731938,
                lat: 59.451348,
            },
            GeographicCoordinates {
                lng: 24.731837,
                lat: 59.451372,
            },
            GeographicCoordinates {
                lng: 24.73182,
                lat: 59.451409,
            },
            GeographicCoordinates {
                lng: 24.731764,
                lat: 59.45143,
            },
            GeographicCoordinates {
                lng: 24.731691,
                lat: 59.45141,
            },
            GeographicCoordinates {
                lng: 24.7316,
                lat: 59.451419,
            },
            GeographicCoordinates {
                lng: 24.731523,
                lat: 59.451441,
            },
            GeographicCoordinates {
                lng: 24.731316,
                lat: 59.451503,
            },
            GeographicCoordinates {
                lng: 24.73025,
                lat: 59.451843,
            },
            GeographicCoordinates {
                lng: 24.730235,
                lat: 59.451886,
            },
            GeographicCoordinates {
                lng: 24.730259,
                lat: 59.451907,
            },
            GeographicCoordinates {
                lng: 24.729829,
                lat: 59.452026,
            },
            GeographicCoordinates {
                lng: 24.729962,
                lat: 59.452169,
            },
            GeographicCoordinates {
                lng: 24.730034,
                lat: 59.452226,
            },
        ];
        assert_eq!(response[0].geometry, expected_coords);
    }
}
