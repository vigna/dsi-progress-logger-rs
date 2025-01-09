/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use dsi_progress_logger::*;
use log::info;
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init()?;

    // Chained-setter initialization
    let mut pl = ProgressLogger::default();
    pl.item_name("pumpkin").log_target("slow smashing");

    pl.start("Smashing pumpkins (slowly)...");
    for _ in 0..30 {
        thread::sleep(std::time::Duration::from_millis(1000));
        pl.update();
    }
    pl.done();

    info!("");

    // Macro initialization
    let mut pl = progress_logger!(
        display_memory = true,
        item_name = "pumpkin",
        local_speed = true,
        log_target = "fast smashing"
    );

    pl.start("Smashing pumpkins...");
    for _ in 0..300 {
        thread::sleep(std::time::Duration::from_millis(100));
        pl.update();
    }
    pl.done();

    Ok(())
}
