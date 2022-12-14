use std::collections::HashMap;
use std::time::Duration;

use rand::Rng;

use bevy::prelude::*;
use bevy_dice::{DiceRollResult, DiceRollStartEvent};
use bevy_kira_audio::prelude::*;
use bevy_mod_picking::{PickingEvent, SelectionEvent};

use crate::board::{draw_board, StackRankDiceGameBoardElement};
use crate::game::{GameLogEntry, SelectedRegion};
use crate::game::{GameState, Region};
use crate::tiered_prng::PrngMapResource;
use crate::ui::{DiceRollUI, StackRankDiceUI};

/// Event that is fired when two regions on a map are entering a clash
#[allow(dead_code)]
pub(crate) struct EventPlayerMoveStart {
    region_1: Region,
    region_2: Region,
    player_1: usize,
    player_2: usize,
}

/// Event that is fired when a clash between two regions on a map is resolved
/// and the winner is determined
#[allow(dead_code)]
pub(crate) struct EventPlayerMoveEnd {
    player_1: usize,
    player_2: usize,
    region_1: Region,
    region_2: Region,
    region_1_dice_result: Vec<usize>,
    region_2_dice_result: Vec<usize>,
}

/// Event that is fired when a played has won a game
pub(crate) struct EventGameOver {
    // An index of a winner
    winner: usize,
}

/// Event that is fired when a turn of a player is started
#[allow(dead_code)]
pub(crate) struct EventTurnStart {
    // An index of a player
    player: usize,
}

/// Event that is fired when a turn of a player is started
#[allow(dead_code)]
pub(crate) struct EventTurnEnd {
    // An index of a player
    player: usize,
}

pub(crate) fn filter_just_selected_event(
    mut event_reader: EventReader<PickingEvent>,
) -> Option<Entity> {
    for event in event_reader.iter() {
        if let PickingEvent::Selection(SelectionEvent::JustSelected(selection_event)) = event {
            return Some(*selection_event);
        }
    }

    None
}

pub(crate) fn event_region_selected(
    mut selected_region: ResMut<SelectedRegion>,
    picking_events: EventReader<PickingEvent>,
    regions: Query<(Entity, &Region)>,
    game_state: Res<GameState>,
    mut event_writer: EventWriter<EventPlayerMoveStart>,
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
                let event = EventPlayerMoveStart {
                    player_1: region_1.owner,
                    player_2: region_2.owner,
                    region_1,
                    region_2,
                };
                event_writer.send(event);
            }
        }

        selected_region.deselect();
    } else {
        selected_region.select(selected_entity.unwrap(), region.clone());
    }
}

#[derive(Component)]
pub(crate) struct DiceRollTimer {
    timer: Timer,
}

pub(crate) fn event_player_move_start(
    mut commands: Commands,
    mut region_clash_event_reader: EventReader<EventPlayerMoveStart>,
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
            region_1_dice_result: Vec::new(),
            region_2_dice_result: Vec::new(),
            turn_counter,
        });

        dice_roll_started_writer.send(dice_roll_started);

        commands.spawn(()).insert(DiceRollTimer {
            timer: Timer::new(Duration::from_secs(3), TimerMode::Once),
        });
    }
}

pub(crate) fn event_dice_roll_result(
    mut dice_rolls: EventReader<DiceRollResult>,
    mut game_state: ResMut<GameState>,
    asset_server: Res<AssetServer>,
    audio: Res<bevy_kira_audio::prelude::Audio>,
) {
    for event in dice_rolls.iter() {
        let last_log_entry = game_state.game_log.last_mut().unwrap();

        audio.play(asset_server.load("sounds/throw.wav"));

        last_log_entry.region_1_dice_result = event.values[0].clone();
        last_log_entry.region_2_dice_result = event.values[1].clone();
    }
}

pub(crate) fn event_dice_rolls_complete(
    mut commands: Commands,
    mut dice_roll_timer_query: Query<(Entity, &mut DiceRollTimer)>,
    mut dice_roll_ui_query: Query<(Entity, &mut Visibility, &mut DiceRollUI)>,
    time: Res<Time>,
    mut region_clash_end_event_writer: EventWriter<EventPlayerMoveEnd>,
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

            region_clash_end_event_writer.send(EventPlayerMoveEnd {
                player_1: last_log_entry.region_1.owner,
                player_2: last_log_entry.region_2.owner,
                region_1: last_log_entry.region_1.clone(),
                region_2: last_log_entry.region_2.clone(),
                region_1_dice_result: last_log_entry.region_1_dice_result.clone(),
                region_2_dice_result: last_log_entry.region_2_dice_result.clone(),
            })
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn event_player_move_end(
    mut region_clash_end_event_reader: EventReader<EventPlayerMoveEnd>,
    mut game_state: ResMut<GameState>,
    mut game_elements_query: Query<(Entity, &StackRankDiceGameBoardElement)>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    map_prng: ResMut<PrngMapResource>,
    materials: ResMut<Assets<StandardMaterial>>,
    mut selected_region: ResMut<SelectedRegion>,
    audio: Res<bevy_kira_audio::prelude::Audio>,
    mut event_game_over_writer: EventWriter<EventGameOver>,
    mut event_turn_end_writer: EventWriter<EventTurnEnd>,
    mut event_turn_start_writer: EventWriter<EventTurnStart>,
) {
    let mut rng = rand::thread_rng();
    let mut redraw_board = false;

    for e in region_clash_end_event_reader.iter() {
        let result_1: usize = e.region_1_dice_result.iter().sum();
        let result_2: usize = e.region_2_dice_result.iter().sum();

        if result_1 > result_2 {
            // win a region
            game_state.board.regions[e.region_2.id].owner = e.region_1.owner;
            if e.region_1.num_dice > 1 {
                game_state.board.regions[e.region_2.id].num_dice =
                    rng.gen_range(1..e.region_1.num_dice);
                game_state.board.regions[e.region_1.id].num_dice -=
                    game_state.board.regions[e.region_2.id].num_dice - 1;
            }

            audio.play(asset_server.load("sounds/win.wav"));
        } else {
            // lose a region
            game_state.board.regions[e.region_1.id].owner = e.region_2.owner;
            if e.region_2.num_dice > 1 {
                game_state.board.regions[e.region_1.id].num_dice =
                    rng.gen_range(1..e.region_2.num_dice);
                game_state.board.regions[e.region_2.id].num_dice -=
                    game_state.board.regions[e.region_1.id].num_dice - 1;
            }

            audio.play(asset_server.load("sounds/loss.wav"));
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

    // check whether it's time to switch turn
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
        event_turn_end_writer.send(EventTurnEnd {
            player: game_state.turn_of_player,
        });

        game_state.turn_of_player += 1;
        if game_state.turn_of_player >= game_state.number_of_players {
            game_state.turn_of_player = 0;
        }

        event_turn_start_writer.send(EventTurnStart {
            player: game_state.turn_of_player,
        });

        game_state.turn_counter += 1;
    }

    // check whether it's time to end the game
    let mut regions_by_player: HashMap<usize, usize> = HashMap::new();
    for region in game_state.board.regions.iter() {
        let current_count = *regions_by_player.get(&region.owner).unwrap_or(&0);
        regions_by_player.insert(region.owner, current_count + 1);
    }

    for (player, number_of_regions) in regions_by_player.iter() {
        if *number_of_regions == game_state.board.regions.len() {
            event_game_over_writer.send(EventGameOver { winner: *player });
            return;
        }
    }

    if redraw_board {
        selected_region.deselect();
        draw_board(
            asset_server,
            commands,
            meshes,
            map_prng,
            materials,
            game_state,
        );
    }
}

pub(crate) fn event_game_over(
    mut commands: Commands,
    mut event_game_over_reader: EventReader<EventGameOver>,
    mut game_elements_query: Query<(Entity, &StackRankDiceGameBoardElement)>,
    mut game_ui_elements_query: Query<(Entity, &StackRankDiceUI)>,
    asset_server: Res<AssetServer>,
    _audio: Res<bevy_kira_audio::prelude::Audio>,
) {
    for e in event_game_over_reader.iter() {
        for (e, _) in game_elements_query.iter_mut() {
            commands.entity(e).despawn_recursive();
        }

        for (e, _) in game_ui_elements_query.iter_mut() {
            commands.entity(e).despawn_recursive();
        }

        commands
            .spawn(
                TextBundle::from_section(
                    format!("Player {} wins!", e.winner + 1),
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 50.0,
                        color: Color::WHITE,
                    },
                )
                .with_text_alignment(TextAlignment::TOP_CENTER)
                .with_style(Style {
                    position_type: PositionType::Absolute,
                    position: UiRect {
                        bottom: Val::Percent(50.0),
                        left: Val::Percent(45.0),
                        ..default()
                    },
                    ..default()
                }),
            )
            .insert(StackRankDiceUI);

        // _audio.play(asset_server.load("sounds/game_over.wav"));
    }
}
