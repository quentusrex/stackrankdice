use std::collections::HashMap;

use bevy::prelude::{Component, Entity, Resource};
use rand::{seq::IteratorRandom, Rng};
use rand_chacha::ChaCha20Rng;

use crate::hex::HexCoord;

const BOARD_SIZE: isize = 20;
const NUMBER_OF_PATCHES: usize = 16;
const HALF_BOARD_SIZE: isize = BOARD_SIZE / 2 - 1;

#[derive(Default, Clone)]
pub struct Board {
    pub hexes: HashMap<(isize, isize), usize>,
    pub regions: Vec<Region>,
}

#[derive(Clone, Resource)]
pub struct GameState {
    pub board: Board,
    pub turn_of_player: usize,
    pub turn_counter: usize,
    pub number_of_players: usize,
    pub game_log: Vec<GameLogEntry>,
}

impl GameState {
    // Enumerates a list of possible moves for a player
    #[allow(dead_code)]
    pub fn possible_moves(self) -> Vec<(Region, Region)> {
        let regions_owned_by_player: Vec<Region> = self
            .board
            .regions
            .iter()
            .filter(|region| region.owner == self.turn_of_player)
            .cloned()
            .collect();

        let mut possible_moves: Vec<(Region, Region)> = Vec::new();

        for region1 in regions_owned_by_player.iter() {
            for region2 in self.board.regions.iter() {
                if region1.is_opponent(region2) {
                    possible_moves.push((region1.clone(), region2.clone()));
                }
            }
        }

        possible_moves
    }
}

#[derive(Clone)]
pub struct GameLogEntry {
    pub turn_counter: usize,
    pub turn_of_player: usize,
    pub region_1: Region,
    pub region_2: Region,
    pub region_1_dice_result: Vec<usize>,
    pub region_2_dice_result: Vec<usize>,
}

#[derive(Default, Component, Clone)]
pub struct Region {
    pub hexes: Vec<(isize, isize)>,
    pub owner: usize,
    pub num_dice: usize,
    pub id: usize,
}

impl Region {
    pub fn center_of_mass(&self) -> (f32, f32) {
        let mut x = 0.0;
        let mut y = 0.0;
        for (hx, hy) in self.hexes.iter() {
            x += *hx as f32;
            y += *hy as f32;
        }

        (x / self.hexes.len() as f32, y / self.hexes.len() as f32)
    }

    pub fn center_hex(&self) -> HexCoord {
        let center = self.center_of_mass();
        let mut nearest_hex: (isize, isize) = (0, 0);
        let mut min_distance = f32::MAX;
        for point in self.hexes.iter() {
            let distance =
                ((center.0 - point.0 as f32).powi(2) + (center.1 - point.1 as f32).powi(2)).sqrt();

            if distance < min_distance {
                min_distance = distance;
                nearest_hex = *point;
            }
        }

        HexCoord::new(nearest_hex.0, nearest_hex.1)
    }

    pub fn is_opponent(&self, other: &Region) -> bool {
        if self.owner == other.owner {
            return false;
        }

        for hex in self.hexes.iter() {
            let hex_coord = HexCoord::new(hex.0, hex.1);
            for neighbour_coord in hex_coord.neighbors() {
                if other
                    .hexes
                    .contains(&(neighbour_coord.q, neighbour_coord.r))
                {
                    return true;
                }
            }
        }

        false
    }
}

pub fn generate_board(number_of_players: usize, mut rng: ChaCha20Rng) -> Board {
    // Roughly half of the board occupied by patches (regions)
    let patch_size: isize =
        (BOARD_SIZE * BOARD_SIZE) / (NUMBER_OF_PATCHES * number_of_players * 2) as isize;

    let mut board = Board::default();

    for patch in 0..NUMBER_OF_PATCHES {
        for player in 0..number_of_players {
            let mut is_starting_point_valid = false;

            while !is_starting_point_valid {
                let mut has_neighbours = false;

                while !has_neighbours {
                    let mut hex_snapshot = board.hexes.clone();

                    // check if starting position is empty
                    let initial_coord = (
                        rng.gen_range(-HALF_BOARD_SIZE..HALF_BOARD_SIZE),
                        rng.gen_range(-HALF_BOARD_SIZE..HALF_BOARD_SIZE),
                    );

                    if board.hexes.get(&initial_coord).is_none() {
                        is_starting_point_valid = true;
                        hex_snapshot.insert(initial_coord, player);
                    } else {
                        // try over
                        continue;
                    }

                    // expand until size limit is reached or no more space to grow
                    let mut patch_hexes: Vec<(isize, isize)> = vec![initial_coord];

                    for _ in 0..patch_size {
                        // find a bordering hex. use random iterating order to avoid bias
                        let mut neightbour_hex: Option<HexCoord> = None;
                        for coord in patch_hexes
                            .iter()
                            .choose_multiple(&mut rng, patch_hexes.iter().len())
                        {
                            let hex = HexCoord::new(coord.0, coord.1);
                            // iterate over all neighbors and find a free one
                            for neighbor in hex.neighbors() {
                                if hex_snapshot.get(&(neighbor.q, neighbor.r)).is_none() {
                                    neightbour_hex = Some(hex.clone());
                                    break;
                                }
                            }

                            // continue expanding a border hex
                            if neightbour_hex.is_some() {
                                break;
                            }
                        }

                        // no more hex cells in this patch
                        if neightbour_hex.is_none() {
                            break;
                        }

                        // add a new hex to the patch
                        let mut candidates: Vec<(isize, isize)> = vec![];
                        for neighbour in neightbour_hex.unwrap().neighbors() {
                            let neighbour_coord = (neighbour.q, neighbour.r);
                            if hex_snapshot.get(&neighbour_coord).is_none() {
                                candidates.push(neighbour_coord);
                            }
                        }
                        let candidate = candidates.iter().choose(&mut rng).unwrap();
                        patch_hexes.push(*candidate);
                        hex_snapshot.insert(*candidate, player);
                    }

                    if patch_hexes.len() == 1 {
                        break;
                    }

                    // check whether patch has any neighbors or start over
                    for patch_hex in patch_hexes.iter() {
                        let hex = HexCoord::new(patch_hex.0, patch_hex.1);
                        for neighbor in hex.neighbors() {
                            if board.hexes.get(&(neighbor.q, neighbor.r)).is_some() {
                                has_neighbours = true;
                                break;
                            }
                        }
                    }

                    // if could not generate a patch with a neightbours, start over
                    // except for the first patch
                    if player == 0 && patch == 0 {
                        has_neighbours = true;
                    }

                    // if patch has neighbours, add it to the board
                    // else, start over
                    if has_neighbours {
                        board.hexes = hex_snapshot;
                        board.regions.push(Region {
                            hexes: patch_hexes,
                            owner: player,
                            num_dice: 0,
                            id: board.regions.len(),
                        });
                        break;
                    }
                }
            }
        }
    }

    // allocate dice
    let mut dice_budget: HashMap<usize, usize> = HashMap::new();
    for p in 0..number_of_players {
        dice_budget.insert(p, NUMBER_OF_PATCHES * 4);
    }

    for region in board.regions.iter_mut() {
        region.num_dice = rng.gen_range(1..usize::min(4, dice_budget[&region.owner]));
        dice_budget.insert(region.owner, dice_budget[&region.owner] - region.num_dice);
    }

    board
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
