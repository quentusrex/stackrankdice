mod game;
mod geometry;
mod hex;
mod highlights;

use std::time::Duration;

use bevy::{
    prelude::*,
    render::{camera::ScalingMode, mesh::Indices, render_resource::PrimitiveTopology},
};
use bevy_dice::{DicePlugin, DicePluginSettings, DiceRollResult, DiceRollStartEvent};
// use bevy_inspector_egui::WorldInspectorPlugin;
use bevy_mod_outline::*;

use bevy_mod_picking::{PickableBundle, PickingCameraBundle, PickingEvent, SelectionEvent};
use bevy_rapier3d::prelude::{NoUserData, RapierPhysicsPlugin};
use game::{generate_board, GameState, Region};
use geometry::center;
use rand::Rng;

use crate::hex::HexCoord;
use crate::{game::GameLogEntry, geometry::flat_hexagon_points};

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

const PLAYER_COLORS: [Color; 8] = [
    Color::PURPLE,
    Color::CYAN,
    Color::GREEN,
    Color::YELLOW,
    Color::RED,
    Color::ORANGE,
    Color::PINK,
    Color::OLIVE,
];

#[derive(Component)]
struct TitleText;

#[derive(Component)]
struct CurrentTurnText;

#[derive(Component)]
struct DiceRollUI;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    dice_plugin_settings: Res<DicePluginSettings>,
) {
    // Camera
    commands
        // camera
        .spawn_bundle(Camera3dBundle {
            projection: OrthographicProjection {
                scaling_mode: ScalingMode::FixedVertical(3.0),

                scale: 10.0,
                ..default()
            }
            .into(),
            camera: Camera {
                priority: 1,
                // target: RenderTarget::Image(image_handle.clone()),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(50.0, 32., 0.0))
                .looking_at(Vec3::default(), Vec3::Y),
            ..Default::default()
        })
        .insert_bundle(PickingCameraBundle::default())
        // .insert(UiCameraConfig { show_ui: false })
        .insert(Name::new("Board Camera"));

    // Current Turn Text
    commands
        .spawn_bundle(
            TextBundle::from_section(
                "current turn",
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 50.0,
                    color: Color::BLACK,
                },
            )
            .with_text_alignment(TextAlignment::TOP_CENTER)
            .with_style(Style {
                position_type: PositionType::Absolute,
                position: UiRect {
                    bottom: Val::Px(5.0),
                    left: Val::Px(15.0),
                    ..default()
                },
                ..default()
            }),
        )
        .insert(CurrentTurnText);

    // Dice Roll camera
    commands.spawn_bundle(Camera2dBundle {
        camera: Camera {
            // priority: 2,
            ..default()
        },
        ..default()
    });

    for (i, dice_camera) in dice_plugin_settings.render_handles.iter().enumerate() {
        commands
            .spawn_bundle(ImageBundle {
                image: UiImage(dice_camera.clone()),
                style: Style {
                    size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                    ..default()
                },
                ..default()
            })
            .insert(Name::new("Dice Roll View"))
            .insert(DiceRollUI)
            .insert(Visibility { is_visible: false });

        // Dice Throw Sum Text
        commands
            .spawn_bundle(
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 150.0,
                        color: Color::WHITE,
                    },
                )
                .with_text_alignment(TextAlignment::TOP_CENTER)
                .with_style(Style {
                    position_type: PositionType::Absolute,
                    position: UiRect {
                        bottom: Val::Percent(50.0),
                        left: Val::Percent(25.0 + 50.0 * (i as f32)),
                        ..default()
                    },
                    ..default()
                }),
            )
            .insert(Name::new("Dice Throw Sum Text"))
            .insert(DiceRollUI)
            .insert(Visibility { is_visible: false });
    }

    // Title Text
    commands
        .spawn_bundle(
            TextBundle::from_section(
                "STACK RANK DICE",
                TextStyle {
                    font: asset_server.load("fonts/HEXAGON_.TTF"),
                    font_size: 80.0,
                    color: Color::WHITE,
                },
            )
            .with_text_alignment(TextAlignment::TOP_CENTER)
            .with_style(Style {
                position_type: PositionType::Absolute,
                position: UiRect {
                    top: Val::Px(5.0),
                    right: Val::Px(15.0),
                    ..default()
                },
                ..default()
            }),
        )
        .insert(TitleText);
}

#[derive(Component)]
struct StackRankDiceGameBoardElement;

fn draw_board(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    game_state: ResMut<GameState>,
) {
    let mut rng = rand::thread_rng();
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
        let height: f32 = 1.0 + rng.gen_range(0.0..=0.0001);
        let mut bundle_command = commands.spawn_bundle(PbrBundle {
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
            .insert_bundle(OutlineBundle {
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
            bundle_command.insert_bundle(PickableBundle::default());
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
                .spawn_bundle(PbrBundle {
                    mesh: dice_mesh_handle.clone(),
                    material: material_handle.clone(),
                    transform: Transform::from_xyz(pos[0], y_pos, z_pos)
                        .with_scale(Vec3::splat(0.4)),
                    ..default()
                })
                .insert(OutlineStencil {})
                .insert(Name::new("Dice"))
                .insert(StackRankDiceGameBoardElement);
        }

        commands
            .spawn_bundle(PointLightBundle {
                transform: Transform::from_xyz(pos[0] + 2.0, 2.0, pos[2]),
                point_light: PointLight {
                    intensity: 100.0,
                    ..default()
                },
                ..default()
            })
            .insert(Name::new("RegionLight"))
            .insert(StackRankDiceGameBoardElement);
    }
}

fn filter_just_selected_event(mut event_reader: EventReader<PickingEvent>) -> Option<Entity> {
    for event in event_reader.iter() {
        if let PickingEvent::Selection(SelectionEvent::JustSelected(selection_event)) = event {
            return Some(*selection_event);
        }
    }

    None
}
fn event_region_selected(
    mut selected_region: ResMut<SelectedRegion>,
    picking_events: EventReader<PickingEvent>,
    regions: Query<(Entity, &Region)>,
    game_state: Res<GameState>,
    mut event_writer: EventWriter<RegionClashEventStart>,
) {
    let selected_entity = filter_just_selected_event(picking_events);

    if selected_entity.is_none() {
        return;
    }

    let region = regions.get(selected_entity.unwrap()).unwrap().1;

    if region.owner != game_state.turn_of_player {
        if selected_region.region.is_some() {
            let region_1 = selected_region.region.clone().unwrap();
            let region_2 = region.clone();
            if region_1.is_opponent(&region_2) {
                // Attack a neighbour
                let event = RegionClashEventStart { region_1, region_2 };
                event_writer.send(event);
            }
        }

        selected_region.deselect();
    } else {
        selected_region.select(selected_entity.unwrap(), region.clone());
    }
}

#[derive(Default)]
pub struct SelectedRegion {
    pub entity: Option<Entity>,
    pub region: Option<Region>,
}

impl SelectedRegion {
    pub fn select(&mut self, entity: Entity, region: Region) {
        self.entity = Some(entity);
        self.region = Some(region);
    }

    pub fn deselect(&mut self) {
        self.entity = None;
    }
}

fn player_turn_text_update(
    game_state: Res<GameState>,
    selected_region: Res<SelectedRegion>,
    mut query: Query<&mut Text, With<CurrentTurnText>>,
) {
    for mut text in &mut query {
        let mut entity_id: String = String::new();

        if selected_region.entity.is_some() {
            entity_id = format!("{}", selected_region.entity.unwrap().id());
        }

        text.sections[0].value = format!(
            "PLAYER {} TURN {}",
            game_state.turn_of_player + 1,
            entity_id
        );
        text.sections[0].style.color = PLAYER_COLORS[game_state.turn_of_player as usize];
    }
}

fn dice_roll_result_text_ui(
    game_state: Res<GameState>,
    mut query: Query<&mut Text, With<DiceRollUI>>,
) {
    for (i, mut text) in &mut query.iter_mut().enumerate() {
        let last_log_entry = game_state.game_log.last();
        if last_log_entry.is_none() {
            return;
        }

        let log_entry = last_log_entry.unwrap();
        if i == 0 && log_entry.dice_1_sum != 0 {
            text.sections[0].value = format!("{}", log_entry.dice_1_sum);
        } else if log_entry.dice_2_sum != 0 {
            text.sections[0].value = format!("{}", log_entry.dice_2_sum);
        }
    }
}

// Events

pub struct RegionClashEventStart {
    region_1: Region,
    region_2: Region,
}

pub struct RegionClashEventEnd {
    region1: Region,
    region2: Region,
    dice_1_sum: usize,
    dice_2_sum: usize,
}

#[derive(Component)]
struct DiceRollTimer {
    timer: Timer,
}

fn event_region_clash(
    mut commands: Commands,
    mut region_clash_event_reader: EventReader<RegionClashEventStart>,
    mut dice_roll_started_writer: EventWriter<DiceRollStartEvent>,
    mut dice_roll_view_query: Query<(Entity, &mut Visibility, &DiceRollUI)>,
    mut game_state: ResMut<GameState>,
) {
    let turn_of_player = game_state.turn_of_player;
    let turn_counter = game_state.turn_counter;

    for event in region_clash_event_reader.iter() {
        // Side 1 roll dice
        let mut dice_roll_started = DiceRollStartEvent {
            num_dice: Vec::new(),
        };

        dice_roll_started.num_dice.push(event.region_1.num_dice);
        dice_roll_started.num_dice.push(event.region_2.num_dice);

        for (_, mut v, _) in dice_roll_view_query.iter_mut() {
            v.is_visible = true;
        }

        game_state.game_log.push(GameLogEntry {
            turn_of_player,
            region_1: event.region_1.clone(),
            region_2: event.region_2.clone(),
            dice_1_sum: 0,
            dice_2_sum: 0,
            turn_counter,
        });

        dice_roll_started_writer.send(dice_roll_started);

        commands.spawn().insert(DiceRollTimer {
            timer: Timer::new(Duration::from_secs(3), false),
        });
    }
}

fn event_dice_roll_result(
    mut dice_rolls: EventReader<DiceRollResult>,
    mut game_state: ResMut<GameState>,
) {
    for event in dice_rolls.iter() {
        let last_log_entry = game_state.game_log.last_mut().unwrap();

        last_log_entry.dice_1_sum = event.values[0].iter().sum();
        last_log_entry.dice_2_sum = event.values[1].iter().sum();
    }
}

fn event_dice_rolls_complete(
    mut commands: Commands,
    mut dice_roll_timer_query: Query<(Entity, &mut DiceRollTimer)>,
    mut dice_roll_ui_query: Query<(Entity, &mut Visibility, &mut DiceRollUI)>,
    time: Res<Time>,
    mut region_clash_end_event_writer: EventWriter<RegionClashEventEnd>,
    mut game_state: ResMut<GameState>,
) {
    for (entity, mut fuse_timer) in dice_roll_timer_query.iter_mut() {
        fuse_timer.timer.tick(time.delta());
        if fuse_timer.timer.finished() {
            commands.entity(entity).despawn();

            for (_, mut v, _) in dice_roll_ui_query.iter_mut() {
                v.is_visible = false;
            }

            let last_log_entry = game_state.game_log.last_mut().unwrap();

            region_clash_end_event_writer.send(RegionClashEventEnd {
                region1: last_log_entry.region_1.clone(),
                region2: last_log_entry.region_2.clone(),
                dice_1_sum: last_log_entry.dice_1_sum,
                dice_2_sum: last_log_entry.dice_2_sum,
            })
        }
    }
}

fn event_region_clash_end(
    mut region_clash_end_event_reader: EventReader<RegionClashEventEnd>,
    mut game_state: ResMut<GameState>,
    mut game_elements_query: Query<(Entity, &StackRankDiceGameBoardElement)>,

    asset_server: Res<AssetServer>,
    mut commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut rng = rand::thread_rng();
    let mut redraw_board = false;

    for e in region_clash_end_event_reader.iter() {
        if e.dice_1_sum > e.dice_2_sum {
            game_state.board.regions[e.region2.id].owner = e.region1.owner;
            if e.region1.num_dice > 1 {
                game_state.board.regions[e.region2.id].num_dice =
                    rng.gen_range(1..e.region1.num_dice);
                game_state.board.regions[e.region1.id].num_dice -=
                    game_state.board.regions[e.region2.id].num_dice
            }
        } else {
            game_state.board.regions[e.region1.id].owner = e.region2.owner;
            if e.region2.num_dice > 1 {
                game_state.board.regions[e.region1.id].num_dice =
                    rng.gen_range(1..e.region2.num_dice);
                game_state.board.regions[e.region2.id].num_dice -=
                    game_state.board.regions[e.region1.id].num_dice
            }
        }

        for (e, _) in game_elements_query.iter_mut() {
            commands.entity(e).despawn_recursive();
        }

        redraw_board = true;
    }

    // check whether it's time to switch turn
    let region_made_move_this_turn: Vec<Region> = game_state
        .game_log
        .iter()
        .filter(|gl| {
            gl.turn_counter == game_state.turn_counter
                && gl.turn_of_player == game_state.turn_of_player
        })
        .map(|gl| gl.region_1.clone())
        .collect();

    let regions_able_to_move_this_turn = game_state.board.regions.iter().filter(|gl| {
        gl.owner == game_state.turn_of_player
            && region_made_move_this_turn
                .iter()
                .filter(|r| r.id == gl.id)
                .count()
                == 0
    });

    let number_of_unblocked_regions = regions_able_to_move_this_turn
        .filter(|r1| {
            game_state
                .board
                .regions
                .iter()
                .filter(|r2| r2.is_opponent(r1))
                .count()
                > 0
        })
        .count();

    if number_of_unblocked_regions == 0 {
        game_state.turn_of_player += 1;
        if game_state.turn_of_player >= game_state.number_of_players {
            game_state.turn_of_player = 0;
        }
        game_state.turn_counter += 1;
    }

    if redraw_board {
        draw_board(asset_server, commands, meshes, materials, game_state);
    }
}

fn main() {
    let number_of_players = 2;

    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugins(highlights::StackRankDicePickingPlugins)
        // .add_plugin(WorldInspectorPlugin::new())
        .add_plugin(OutlinePlugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(DicePlugin)
        .insert_resource(DicePluginSettings {
            render_size: (640 * 2, 720 * 2),
            number_of_fields: 2,
            ..default()
        })
        .add_startup_system(setup.after("dice_plugin_init").label("setup"))
        .add_startup_system(draw_board.after("setup"))
        .add_system(player_turn_text_update)
        .add_system_to_stage(CoreStage::PostUpdate, event_region_selected)
        .add_system(event_region_clash)
        .add_system(event_dice_roll_result)
        .add_system(dice_roll_result_text_ui)
        .add_system(event_dice_rolls_complete)
        .add_system(event_region_clash_end)
        .insert_resource(GameState {
            board: generate_board(number_of_players),
            number_of_players,
            turn_of_player: 0,
            turn_counter: 0,
            game_log: Vec::new(),
        })
        .init_resource::<SelectedRegion>()
        .add_event::<RegionClashEventStart>()
        .add_event::<RegionClashEventEnd>()
        .run();
}
