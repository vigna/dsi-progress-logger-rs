/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 * SPDX-FileCopyrightText: 2024 Fondation Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use dsi_progress_logger::*;

const ITER_PER_THREAD: u64 = 1000000000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init()?;

    // Convenience macro
    let mut cpl = concurrent_progress_logger![
        item_name = "pumpkin",
        expected_updates = (100 * ITER_PER_THREAD).try_into().ok()
    ];

    cpl.start(format!(
        "Smashing {} pumpkins (using many threads)...",
        100 * ITER_PER_THREAD
    ));

    std::thread::scope(|s| {
        for i in 0..100 {
            let mut pl = cpl.clone();
            s.spawn(move || {
                let i = i as u64;
                for _ in (i * ITER_PER_THREAD)..(i + 1) * ITER_PER_THREAD {
                    pl.update();
                }
            });
        }
    });
    cpl.done();

    Ok(())
}
