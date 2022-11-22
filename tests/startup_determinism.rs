use bevy::prelude::*;
use rand::Rng;
use stackrankdice::app::build_app;
use stackrankdice::tiered_prng::PrngMapResource;

#[test]
fn fixed_world_undef_env_seed() {
    // Setup app
    let mut app = App::new();
    build_app(&mut app, 4242, 0, 2, true);

    let mut map_prng = app.world.get_resource_mut::<PrngMapResource>().unwrap();

    let first: f32 = map_prng.rng.gen_range(0.0..=0.0001);
    assert_eq!(first, 2.1680013e-5);
}
