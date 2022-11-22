use bevy::{prelude::*, render::camera::ScalingMode};
use bevy_dice::DicePluginSettings;
use bevy_kira_audio::prelude::*;
use bevy_mod_picking::PickingCameraBundle;

use crate::board::PLAYER_COLORS;
use crate::game::GameState;

/// Text area with a title for the game
#[derive(Component)]
pub(crate) struct TitleText;

/// Text area with current turn counter
#[derive(Component)]
pub(crate) struct CurrentTurnText;

/// UI elements associated with dice rolling
#[derive(Component)]
pub(crate) struct DiceRollUI;

/// UI element for a game. Used for end-game screen to destroy all UI elements
#[derive(Component)]
pub(crate) struct StackRankDiceUI;

pub(crate) fn player_turn_text_update(
    game_state: Res<GameState>,
    mut query: Query<&mut Text, With<CurrentTurnText>>,
) {
    for mut text in &mut query {
        text.sections[0].value = format!("PLAYER {} TURN", game_state.turn_of_player + 1,);
        text.sections[0].style.color = PLAYER_COLORS[game_state.turn_of_player as usize];
    }
}

pub(crate) fn dice_roll_result_text_update(
    game_state: Res<GameState>,
    mut query: Query<&mut Text, With<DiceRollUI>>,
) {
    for (i, mut text) in &mut query.iter_mut().enumerate() {
        let last_log_entry = game_state.game_log.last();
        if last_log_entry.is_none() {
            return;
        }

        let log_entry = last_log_entry.unwrap();
        let result_1: usize = log_entry.region_1_dice_result.iter().sum();
        let result_2: usize = log_entry.region_2_dice_result.iter().sum();

        if i == 0 && result_1 != 0 {
            text.sections[0].value = format!("{}", result_1);
        } else if result_2 != 0 {
            text.sections[0].value = format!("{}", result_2);
        }
    }
}

pub(crate) fn setup_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    dice_plugin_settings: Res<DicePluginSettings>,
    audio: Res<bevy_kira_audio::prelude::Audio>,
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
        .insert(CurrentTurnText)
        .insert(StackRankDiceUI);

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
            .insert(Visibility { is_visible: false })
            .insert(StackRankDiceUI);

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
            .insert(StackRankDiceUI)
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

    // Music

    audio
        .play(asset_server.load("sounds/laidback.ogg"))
        .looped();
}
