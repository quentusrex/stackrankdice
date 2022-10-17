use std::collections::HashMap;

use rand::seq::IteratorRandom;
use rand::Rng;

use crate::hex::HexCoord;
use crate::PLAYER_COLORS;

const BOARD_SIZE: isize = 20;
// const NUMBER_OF_PLAYERS: usize = 2;
const NUMBER_OF_PATCHES: usize = 16;

#[derive(Default)]
pub(crate) struct Board {
    pub(crate) hexes: HashMap<(isize, isize), usize>,
    pub(crate) regions: Vec<Region>,
}

#[derive(Default)]
pub(crate) struct Region {
    pub(crate) hexes: Vec<(isize, isize)>,
    #[allow(dead_code)]
    pub(crate) owner: usize,
    pub(crate) number_of_dice: usize,
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
}

pub(crate) fn generate_board(number_of_players: usize) -> Board {
    const HALF_BOARD_SIZE: isize = BOARD_SIZE / 2 - 1;
    // Roughly half of the board occupied by patches (squads)
    let patch_size: isize =
        (BOARD_SIZE * BOARD_SIZE) / (NUMBER_OF_PATCHES * number_of_players * 2) as isize;

    let mut board = Board::default();
    let mut rng = rand::thread_rng();

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
                            number_of_dice: 0,
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
        region.number_of_dice = rng.gen_range(1..usize::min(4, dice_budget[&region.owner]));
        dice_budget.insert(
            region.owner,
            dice_budget[&region.owner] - region.number_of_dice,
        );
    }

    board
}
