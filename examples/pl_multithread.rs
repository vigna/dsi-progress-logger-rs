/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 * SPDX-FileCopyrightText: 2024 Fondation Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use std::sync::{Arc, Mutex};

use dsi_progress_logger::*;
use rayon::prelude::*;
use stderrlog;

const ITER_PER_THREAD: u64 = 1000000000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    stderrlog::new()
        .verbosity(2)
        .timestamp(stderrlog::Timestamp::Second)
        .init()?;

    // With raw threads
    let mut pl = ProgressLogger::default();
    pl.item_name("pumpkin");
    pl.start("Smashing pumpkins (with raw threads)...");

    std::thread::scope(|s| {
        let buffered_pl = BufferedProgressLogger::new(Arc::new(Mutex::new(&mut pl)));
        for i in 0..num_cpus::get() {
            let mut buffered_pl = buffered_pl.clone();
            s.spawn(move || {
                let i = i as u64;
                for _ in (i * ITER_PER_THREAD)..(i + 1) * ITER_PER_THREAD {
                    buffered_pl.light_update();
                }
            });
        }
    });
    pl.done();

    // With rayon
    let mut pl = progress_logger!(
        item_name = "pumpkin",
        display_memory = true,
        local_speed = true
    );
    pl.start("Smashing pumpkins (with rayon)");

    (0..(num_cpus::get() as u64) * ITER_PER_THREAD)
        .into_par_iter()
        .for_each_with(
            BufferedProgressLogger::new(Arc::new(Mutex::new(&mut pl))),
            |buffered_pl, _| buffered_pl.light_update(),
        );
    pl.done();

    Ok(())
}
