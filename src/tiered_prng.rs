use bevy::prelude::*;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

#[derive(Resource)]
pub struct PrngResource {
    pub world_seed: u64,
    pub env_seed: u64,
}

pub struct PrngPlugin;

#[derive(Resource)]
pub struct PrngMapResource {
    pub rng: ChaCha20Rng,
}

impl Plugin for PrngPlugin {
    fn build(&self, app: &mut App) {
        let seeds = app.world.get_resource::<PrngResource>().unwrap();

        app.insert_resource(PrngMapResource {
            rng: get_randomness(seeds.world_seed),
        });
    }
}

impl FromWorld for PrngResource {
    fn from_world(_world: &mut World) -> Self {
        // Values zero will be considered uninitialized

        //world.insert_resource(); // MapPrng
        //world.insert_resource(); // AiPrng
        PrngResource {
            world_seed: 0,
            env_seed: 0,
        }
    }
}

pub fn get_randomness(seed: u64) -> ChaCha20Rng {
    ChaCha20Rng::seed_from_u64(seed)
}
