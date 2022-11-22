use bevy::prelude::*;
use stackrankdice::{app::build_app, game::GameState};

#[test]
fn fixed_world_undef_env_seed() {
    // Setup app
    let mut app = App::new();
    build_app(&mut app, 4242, 0, 2, true);

    let game_state = app.world.get_resource::<GameState>().unwrap().clone();

    let possible_moves = game_state.possible_moves();

    println!("Number of possible moves: {:?}", possible_moves.len());
    assert!(!possible_moves.is_empty());
}
