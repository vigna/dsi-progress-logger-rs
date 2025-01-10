/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 * SPDX-FileCopyrightText: 2024 Fondation Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use dsi_progress_logger::*;

const ITER_PER_THREAD: u64 = 100000000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init()?;

    // Convenience macro
    let mut cpl = concurrent_progress_logger![item_name = "pumpkin"];
    cpl.start("Smashing pumpkins (using many threads)...");

    std::thread::scope(|s| {
        for i in 0..100 {
            let mut pl = cpl.clone();
            s.spawn(move || {
                let i = i as u64;
                for _ in (i * ITER_PER_THREAD)..(i + 1) * ITER_PER_THREAD {
                    pl.light_update();
                }
            });
        }
    });
    cpl.done();

    Ok(())
}
