use dsi_progress_logger::ProgressLogger;
use log::info;
use std::thread;
use stderrlog;

fn main() {
    stderrlog::new()
        .verbosity(2)
        .timestamp(stderrlog::Timestamp::Second)
        .init()
        .unwrap();

    let mut pl = ProgressLogger::default();
    pl.item_name = "pumpkin".to_string();
    pl.start("Smashing pumpkins (slowly)...");
    for _ in 0..30 {
        thread::sleep(std::time::Duration::from_millis(1000));
        pl.update();
    }
    pl.done();

    info!("");

    let mut pl = ProgressLogger::default().display_memory();
    pl.item_name = "pumpkin".to_string();
    pl.local_speed = true;
    pl.start("Smashing pumpkins (slowly) and showing memory and local speed...");
    for _ in 0..30 {
        thread::sleep(std::time::Duration::from_millis(1000));
        pl.update();
    }
    pl.done();
}
