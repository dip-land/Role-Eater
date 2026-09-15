use poise::Command;

use crate::{Data, Error};

pub mod stats;
pub mod utility;

pub fn get_commands() -> Vec<Command<Data, Error>> {
    vec![
        stats::leaderboard::leaderboard(),
        stats::me::me(),
        stats::stats_command::stats(),
    ]
}
