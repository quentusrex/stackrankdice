use rand::Rng;

use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};

use bevy_mod_outline::*;
use bevy_mod_picking::PickableBundle;

use crate::geometry::{center, flat_hexagon_points};
use crate::hex::HexCoord;
use crate::tiered_prng::PrngMapResource;
use crate::{
    game::{GameState, Region},
    geometry,
};

pub(crate) const PLAYER_COLORS: [Color; 8] = [
    Color::PURPLE,
    Color::CYAN,
    Color::GREEN,
    Color::YELLOW,
    Color::RED,
    Color::ORANGE,
    Color::PINK,
    Color::OLIVE,
];

/// Generate a single hex mesh
fn generate_hex_region_mesh(region: &Region) -> Mesh {
    let hexes = region.hexes.clone();
    let center = center(1.0, &region.center_hex(), &[0.0, 0.0, 0.0]);

    let mut pts: Vec<[f32; 3]> = vec![];
    let mut normals: Vec<[f32; 3]> = vec![];
    let mut uvs: Vec<[f32; 2]> = vec![];
    let mut indices: Vec<u32> = vec![];

    for (hex_num, hex) in hexes.iter().enumerate() {
        let c = HexCoord::new(hex.0, hex.1);
        let hex_num = hex_num as u32;

        // Populate the points for the top face, as a slightly scaled hexagon
        flat_hexagon_points(&mut pts, 1.0, &c);
        for _ in 0..9 {
            normals.push([0., 1., 0.]);
        }
        for i in 0..=6 {
            indices.push(18 * hex_num); // Center
            indices.push(18 * hex_num + i + 1); // Point       East           North-east
            indices.push(18 * hex_num + i + 2); // Next point  North-east     North-west
        }

        // Adjust location and duplicate points with an offset as a bottom face
        for p in pts.len() - 9..pts.len() {
            pts[p][0] -= center[0];
            pts[p][1] -= center[1];
            pts[p][2] -= center[2];
            pts.push([pts[p][0], pts[p][1] - 0.0001, pts[p][2]]);
        }
        for _ in 0..9 {
            normals.push([0., -1., 0.]);
        }

        // Populate indices for bottom
        for i in 0..=6 {
            indices.push(18 * hex_num + 9); // Center
            indices.push(18 * hex_num + i + 1 + 9); // Point       East           North-east
            indices.push(18 * hex_num + i + 2 + 9); // Next point  North-east     North-west
        }

        // Populate indices sides
        for i in 0..=6 {
            indices.push(18 * hex_num + i + 2);
            indices.push(18 * hex_num + i + 1 + 9);
            indices.push(18 * hex_num + i + 2 + 9);

            indices.push(18 * hex_num + i + 2);
            indices.push(18 * hex_num + i + 1);
            indices.push(18 * hex_num + i + 1 + 9);
        }

        // Finally, UVs
        for _ in 0..18 {
            uvs.push([1.0, 1.0]);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.set_indices(Some(Indices::U32(indices)));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pts);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh
}

#[derive(Component)]
pub(crate) struct StackRankDiceGameBoardElement;

pub(crate) fn draw_board(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut map_prng: ResMut<PrngMapResource>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    game_state: ResMut<GameState>,
) {
    let board = game_state.board.clone();

    // Draw board
    for region in board.regions.iter() {
        let color = PLAYER_COLORS[region.owner as usize];

        let center_coord = center(1.0, &region.center_hex(), &[0.0, 0.0, 0.0]);

        #[allow(clippy::search_is_some)]
        let is_region_playable = game_state
            .game_log
            .iter()
            .find(|gl| {
                gl.turn_counter == game_state.turn_counter
                    && gl.region_1.id == region.id
                    && region.owner == game_state.turn_of_player
            })
            .is_none();

        let material = match is_region_playable {
            true => materials.add(StandardMaterial {
                base_color: color,
                metallic: 0.0,
                reflectance: 0.0,
                ..default()
            }),
            _ => materials.add(StandardMaterial {
                base_color: color + Color::rgba(0.2, 0.2, 0.2, 0.9),
                metallic: 0.0,
                reflectance: 0.0,
                ..default()
            }),
        };

        let mut mesh = generate_hex_region_mesh(region);
        mesh.generate_outline_normals().unwrap();
        let mesh = meshes.add(mesh);
        // Theese micro-height differences are to make otline rendering visible.
        // Otherwise tiles with the same height will be rendered as one.
        let height: f32 = 1.0 + map_prng.rng.gen_range(0.0..=0.0001);
        let mut bundle_command = commands.spawn(PbrBundle {
            mesh: mesh.clone(),
            material: material.clone(),
            transform: Transform::from_translation(Vec3::new(
                center_coord[0],
                center_coord[1] + height,
                center_coord[2],
            )),
            ..Default::default()
        });

        bundle_command
            .insert(OutlineBundle {
                outline: Outline {
                    visible: true,
                    colour: Color::rgba(0.0, 0.0, 0.0, 1.0),
                    width: 0.5,
                },
                ..default()
            })
            .insert(region.clone())
            .insert(Name::new("Hex"))
            .insert(StackRankDiceGameBoardElement);

        if is_region_playable {
            bundle_command.insert(PickableBundle::default());
        }
    }

    // Place dice on areas
    let dice_mesh_handle = asset_server.load("models/dice/scene.gltf#Mesh0/Primitive0");
    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("models/dice/textures/Dice_baseColor.png")),
        normal_map_texture: Some(asset_server.load("models/dice/textures/Dice_normal.png")),
        metallic_roughness_texture: Some(
            asset_server.load("models/dice/textures/Dice_metallicRoughness.png"),
        ),
        ..default()
    });

    for region in board.regions.iter() {
        let center_hex = region.center_hex();
        let pos = geometry::center(1.0, &center_hex, &[0., 0.0, 0.]);

        for i in 0..region.num_dice {
            let mut y_pos = 1.0 + pos[1] + 0.383 + (i as f32) * (2.0 * 0.383);
            let mut z_pos = pos[2];
            if i > 3 {
                y_pos = pos[1] + 0.383 + ((i - 4) as f32) * (2.0 * 0.383);
                z_pos += 0.383 * 2.0 + 0.01;
            }

            if region.num_dice > 3 {
                z_pos -= 0.383;
            }

            commands
                .spawn(PbrBundle {
                    mesh: dice_mesh_handle.clone(),
                    material: material_handle.clone(),
                    transform: Transform::from_xyz(pos[0], y_pos, z_pos)
                        .with_scale(Vec3::splat(0.4)),
                    ..default()
                })
                .insert(OutlineStencil { offset: 1.0 })
                .insert(Name::new("Dice"))
                .insert(StackRankDiceGameBoardElement);
        }

        commands
            .spawn(PointLightBundle {
                point_light: PointLight {
                    intensity: 100.0,
                    ..Default::default()
                },
                transform: Transform::from_xyz(pos[0] + 2.0, 2.0, pos[2]),
                ..Default::default()
            })
            .insert(Name::new("RegionLight"))
            .insert(StackRankDiceGameBoardElement);
    }
}
