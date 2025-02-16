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

    // Convenience macro
    let mut pl = progress_logger![item_name = "pumpkin"];

    pl.start("Smashing pumpkins (slowly)...");
    for _ in 0..30 {
        thread::sleep(std::time::Duration::from_millis(1000));
        pl.update();
    }
    pl.done();

    info!("");

    // Macro initialization
    let mut pl = progress_logger!(
        item_name = "pumpkin",
        display_memory = true,
        local_speed = true
    );

    pl.start("Smashing pumpkins (slowly), showing memory and local speed...");
    for _ in 0..30 {
        thread::sleep(std::time::Duration::from_millis(1000));
        pl.update();
    }
    pl.done();

    pl.debug(format_args!("This is a messagge at debug level"));
    pl.info(format_args!("This is a messagge at info level"));
    pl.warn(format_args!("This is a messagge at warn level"));
    pl.error(format_args!("This is a messagge at error level"));

    Ok(())
}
