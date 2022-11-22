mod events;
mod game;
mod geometry;
mod hex;
mod highlights;
mod tiered_prng;

use clap::Parser;
use rand::rngs::OsRng;
use rand::Rng;
use rand::RngCore;

use bevy::{
    prelude::*,
    render::{camera::ScalingMode, mesh::Indices, render_resource::PrimitiveTopology},
};
use bevy_dice::{DicePlugin, DicePluginSettings};
use bevy_kira_audio::prelude::*;
use bevy_mod_outline::*;
use bevy_mod_picking::{PickableBundle, PickingCameraBundle};
use bevy_rapier3d::prelude::{NoUserData, RapierPhysicsPlugin};

use crate::events::*;
use crate::game::{generate_board, GameState, Region};
use crate::geometry::{center, flat_hexagon_points};
use crate::hex::HexCoord;
use crate::tiered_prng::{get_randomness, PrngMapResource};

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

#[derive(Component)]
struct StackRankDiceUI;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    dice_plugin_settings: Res<DicePluginSettings>,
    audio: Res<bevy_kira_audio::prelude::Audio>,
) {
    // Camera
    commands
        // camera
        .spawn(Camera3dBundle {
            projection: OrthographicProjection {
                scaling_mode: ScalingMode::FixedVertical(3.0),
                scale: 10.0,
                ..default()
            }
            .into(),
            camera: Camera {
                priority: 1,
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(50.0, 32., 0.0))
                .looking_at(Vec3::default(), Vec3::Y),
            ..Default::default()
        })
        .insert(PickingCameraBundle::default())
        // .insert(UiCameraConfig { show_ui: false })
        .insert(Name::new("Board Camera"));

    // Current Turn Text
    commands
        .spawn(
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
        .insert(CurrentTurnText)
        .insert(StackRankDiceUI);

    // Dice Roll camera
    commands.spawn(Camera2dBundle {
        camera: Camera {
            // priority: 2,
            ..default()
        },
        ..default()
    });

    for (i, dice_camera) in dice_plugin_settings.render_handles.iter().enumerate() {
        commands
            .spawn(ImageBundle {
                image: UiImage(dice_camera.clone()),
                style: Style {
                    size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                    ..default()
                },
                ..default()
            })
            .insert(Name::new("Dice Roll View"))
            .insert(DiceRollUI)
            .insert(Visibility { is_visible: false })
            .insert(StackRankDiceUI);

        // Dice Throw Sum Text
        commands
            .spawn(
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
            .insert(StackRankDiceUI)
            .insert(Visibility { is_visible: false });
    }

    // Title Text
    commands
        .spawn(
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

    // Music

    audio
        .play(asset_server.load("sounds/laidback.ogg"))
        .looped();
}

#[derive(Component)]
pub(crate) struct StackRankDiceGameBoardElement;

fn draw_board(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut map_prng: ResMut<PrngMapResource>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    game_state: ResMut<GameState>,
) {
    //    let mut rng = rand::thread_rng();
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

        let mesh = generate_hex_region_mesh(region);
        // mesh.generate_outline_normals().unwrap();
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
                    // colour: Color::rgba(0.0, 0.0, 0.0, 1.0).into(),
                    width: 0.5,
                    ..default()
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

#[derive(Default, Resource)]
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
        self.region = None;
    }
}

fn player_turn_text_update(
    game_state: Res<GameState>,
    mut query: Query<&mut Text, With<CurrentTurnText>>,
) {
    for mut text in &mut query {
        text.sections[0].value = format!("PLAYER {} TURN", game_state.turn_of_player + 1,);
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

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = 0)]
    world_seed: u64,

    #[arg(short, long, default_value_t = 0)]
    env_seed: u64,
}

fn main() {
    let number_of_players = 2;

    let mut args = Args::parse();

    if args.world_seed == 0 || args.env_seed == 0 {
        let mut key = [0u8; 16];
        OsRng.fill_bytes(&mut key);

        // If one, or the other is set, only generate for the unset one.
        // This will allow easier testing later, for fixed world random env_seed.
        // Or for specific AI testing, fixed env_seed but random world.
        if args.world_seed == 0 {
            args.world_seed = OsRng.next_u64();
        }
        if args.env_seed == 0 {
            args.env_seed = OsRng.next_u64();
        }
    }

    // Source of randomness for the game
    let prng_resource = tiered_prng::PrngResource {
        world_seed: args.world_seed,
        env_seed: args.env_seed,
    };

    // Generate game map
    let map = generate_board(number_of_players, get_randomness(prng_resource.world_seed));

    App::new()
        // PRNG setup
        .insert_resource(prng_resource)
        // Plugins
        .add_plugin(tiered_prng::PrngPlugin) // Adds Prng based resources for subcomponents
        .add_plugins(DefaultPlugins)
        .add_plugin(bevy_kira_audio::prelude::AudioPlugin)
        .add_plugins(highlights::StackRankDicePickingPlugins)
        .add_plugin(OutlinePlugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(DicePlugin)
        // Resources
        .insert_resource(DicePluginSettings {
            render_size: (640 * 2, 720 * 2),
            number_of_fields: 2,
            ..default()
        })
        .insert_resource(GameState {
            board: map,
            number_of_players,
            turn_of_player: 0,
            turn_counter: 0,
            game_log: Vec::new(),
        })
        .insert_resource(ClearColor(Color::BLACK))
        .init_resource::<SelectedRegion>()
        // Startup Systems
        .add_startup_system(setup.after("dice_plugin_init").label("setup"))
        .add_startup_system(draw_board.after("setup"))
        // UI Systems
        .add_system(player_turn_text_update)
        .add_system(dice_roll_result_text_ui)
        // Control Handling
        .add_system_to_stage(CoreStage::PostUpdate, event_region_selected)
        // Event Handlers
        .add_system(event_region_clash)
        .add_system(event_dice_roll_result)
        .add_system(event_dice_rolls_complete)
        .add_system(event_region_clash_end)
        .add_system(event_game_over)
        // Events
        .add_event::<EventRegionClashStart>()
        .add_event::<EventRegionClashEnd>()
        .add_event::<EventGameOver>()
        // Ignite Engine
        .run();
}
