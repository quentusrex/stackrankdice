use bevy::prelude::*;
use bevy_dice::{DicePlugin, DicePluginSettings};
use bevy_mod_outline::*;
use bevy_rapier3d::prelude::{NoUserData, RapierPhysicsPlugin};

use crate::board::draw_board;
use crate::game::{generate_board, GameState, SelectedRegion};
use crate::tiered_prng::get_randomness;
use crate::ui::{dice_roll_result_text_update, player_turn_text_update, setup_ui};
use crate::{events::*, highlights, tiered_prng};

pub fn build_app(
    app: &mut App,
    world_seed: u64,
    env_seed: u64,
    number_of_players: usize,
    testing: bool,
) {
    // Generate game map
    let map = generate_board(number_of_players, get_randomness(world_seed));

    // Source of randomness for the game
    let prng_resource = tiered_prng::PrngResource {
        world_seed,
        env_seed,
    };

    if !testing {
        app.add_plugins(DefaultPlugins);
        app.add_plugin(bevy_kira_audio::prelude::AudioPlugin);
        app.add_plugin(OutlinePlugin);
        app.add_plugins(highlights::StackRankDicePickingPlugins);
    }

    app
        // PRNG setup
        .insert_resource(prng_resource)
        // Plugins
        .add_plugin(tiered_prng::PrngPlugin) // Adds Prng based resources for subcomponents
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
        .add_startup_system(setup_ui.after("dice_plugin_init").label("setup"))
        .add_startup_system(draw_board.after("setup"))
        // UI Systems
        .add_system(player_turn_text_update)
        .add_system(dice_roll_result_text_update)
        // Control Handling
        .add_system_to_stage(CoreStage::PostUpdate, event_region_selected)
        // Event Handlers
        .add_system(event_player_move_start)
        .add_system(event_dice_roll_result)
        .add_system(event_dice_rolls_complete)
        .add_system(event_player_move_end)
        .add_system(event_game_over)
        // Events
        .add_event::<EventPlayerMoveStart>()
        .add_event::<EventPlayerMoveEnd>()
        .add_event::<EventGameOver>()
        .add_event::<EventTurnStart>()
        .add_event::<EventTurnEnd>();
}
