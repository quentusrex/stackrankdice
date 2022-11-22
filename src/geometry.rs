// This code taken from https://github.com/Quantumplation/bevy-hex-example

use crate::hex::HexCoord;

/// The ratio between a circle touching the points of a hex grid (the outer radius),
/// and a circle touching the edges of a hex grid (the inner radius).
/// Calculated as sqrt(3) / 2;
pub const HEX_INNER_RADIUS_RATIO: f32 = 0.866_025_4;

/// Generate a point located at the center of a hexagon at `c`, on a grid with hexagons of size `radius`, shifted by `offset`.
/// The parameters are used to compose larger effects like beveling
pub fn center(radius: f32, c: &HexCoord, offset: &[f32; 3]) -> [f32; 3] {
    // Get floating point hex-coords
    let (qf, rf) = (c.q as f32, c.r as f32);
    // We need an outer and inner radius
    let (outer, inner) = (radius, radius * HEX_INNER_RADIUS_RATIO);

    // Start from our q coordinate,
    let start = qf;
    // Shift over by half a unit for each row
    let row_adjustment = 0.5 * rf;
    // This produces a rhombus, use integer division to cancel this out on every other row and get "roughly" a grid
    let rhombus_adjustment = -(c.r / 2) as f32;
    // Scale the whole thing up by twice the inner radius to get our x coordinate
    let x = (start + row_adjustment + rhombus_adjustment) * inner * 2.;
    // Each row moves us by 1.5 times the outer radius along the z axis
    let z = rf * outer * 1.5;

    // Return (x,0,z) shifted by the provided offset
    [x + offset[0], 0. + offset[1], z + offset[2]]
}

/// Generate a pointed located at the eastern corner of a hexagon at `c`, on a grid with hexagons of size `radius`, shifted by `offset`
pub fn east_corner(radius: f32, c: &HexCoord, offset: &[f32; 3]) -> [f32; 3] {
    // Start from the center of our hexagon
    let center = center(radius, c, offset);
    // And move along the z axis for "east" by our radius
    [center[0] + 0., center[1] + 0., center[2] + radius]
}

/// Generate a pointed located at the western corner of a hexagon at `c`, on a grid with hexagons of size `radius`, shifted by `offset`
pub fn west_corner(radius: f32, c: &HexCoord, offset: &[f32; 3]) -> [f32; 3] {
    // Start from the center of our hexagon
    let center = center(radius, c, offset);
    // And move along the z axis for "west" by our radius
    [center[0] + 0., center[1] + 0., center[2] - radius]
}

/// Generate a pointed located at the north-east corner of a hexagon at `c`, on a grid with hexagons of size `radius`, shifted by `offset`
pub fn north_east_corner(radius: f32, c: &HexCoord, offset: &[f32; 3]) -> [f32; 3] {
    // Start from the center of our hexagon
    let center = center(radius, c, offset);
    let inner = radius * HEX_INNER_RADIUS_RATIO;
    // And move along the x axis (for "north") to be aligned with the top edge (i.e. the inner radius)
    // and along the z axis (for "east"), but not as far as the east corner
    [center[0] + inner, center[1] + 0., center[2] + 0.5 * radius]
}

/// Generate a pointed located at the north-west corner of a hexagon at `c`, on a grid with hexagons of size `radius`, shifted by `offset`
pub fn north_west_corner(radius: f32, c: &HexCoord, offset: &[f32; 3]) -> [f32; 3] {
    // Start from the center of our hexagon
    let center = center(radius, c, offset);
    let inner = radius * HEX_INNER_RADIUS_RATIO;
    // And move along the x axis (for "north") to be aligned with the top edge (i.e. the inner radius)
    // and along the z axis (for "west"), but not as far as the east corner
    [center[0] + inner, center[1] + 0., center[2] - 0.5 * radius]
}

/// Generate a pointed located at the south-east corner of a hexagon at `c`, on a grid with hexagons of size `radius`, shifted by `offset`
pub fn south_east_corner(radius: f32, c: &HexCoord, offset: &[f32; 3]) -> [f32; 3] {
    // Start from the center of our hexagon
    let center = center(radius, c, offset);
    let inner = radius * HEX_INNER_RADIUS_RATIO;
    // And move along the x axis (for "south") to be aligned with the top edge (i.e. the inner radius)
    // and along the z axis (for "east"), but not as far as the east corner
    [center[0] - inner, center[1] + 0., center[2] + 0.5 * radius]
}

/// Generate a pointed located at the south-west corner of a hexagon at `c`, on a grid with hexagons of size `radius`, shifted by `offset`
pub fn south_west_corner(radius: f32, c: &HexCoord, offset: &[f32; 3]) -> [f32; 3] {
    // Start from the center of our hexagon
    let center = center(radius, c, offset);
    let inner = radius * HEX_INNER_RADIUS_RATIO;
    // And move along the x axis (for "south") to be aligned with the top edge (i.e. the inner radius)
    // and along the z axis (for "west"), but not as far as the east corner
    [center[0] - inner, center[1] + 0., center[2] - 0.5 * radius]
}

/// Fill `pts` with the points around the edge of a flat hexagon of a specific radius at a specific coordinate
pub fn flat_hexagon_ring(pts: &mut Vec<[f32; 3]>, radius: f32, c: &HexCoord, offset: &[f32; 3]) {
    pts.extend(
        [
            east_corner(radius, c, offset), // Each of the corners, counter-clockwise from the east corner
            north_east_corner(radius, c, offset), // ...
            north_west_corner(radius, c, offset), // ...
            west_corner(radius, c, offset), // ...
            south_west_corner(radius, c, offset), // ...
            south_east_corner(radius, c, offset), // ...
            east_corner(radius, c, offset), // We include the east corner an extra time,
                                            // so we don't have to mess around with modulus
        ]
        .iter(),
    );
}

/// Fill `pts` with the points of a flat hexagon of a specific radius at a specific coordinate
pub fn flat_hexagon_points(pts: &mut Vec<[f32; 3]>, radius: f32, c: &HexCoord) {
    // We'll create 6 triangles, all sharing a center point
    pts.push(center(radius, c, &[0., 0., 0.]));
    flat_hexagon_ring(pts, radius, c, &[0., 0., 0.]);
    pts.push(center(radius, c, &[0., 0., 0.]));
}
