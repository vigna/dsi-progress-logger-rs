/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use dsi_progress_logger::*;
use log::info;
use std::thread;
use stderrlog;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    stderrlog::new()
        .verbosity(2)
        .timestamp(stderrlog::Timestamp::Second)
        .init()?;

    // Chained-setter initialization
    let mut pl = ProgressLogger::default();
    pl.item_name("pumpkin");

    pl.start("Smashing pumpkins (slowly)...");
    for _ in 0..30 {
        thread::sleep(std::time::Duration::from_millis(1000));
        pl.update();
    }
    pl.done();

    info!("");

    // Macro initialization
    let mut pl = progress_logger![
        item_name = "pumpkin",
        display_memory = true,
        local_speed = true
    ];

    pl.start("Smashing pumpkins (slowly) and showing memory and local speed...");
    for _ in 0..30 {
        thread::sleep(std::time::Duration::from_millis(1000));
        pl.update();
    }
    pl.done();

    Ok(())
}
