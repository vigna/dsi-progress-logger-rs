/*
 * SPDX-FileCopyrightText: 2024 Fondation Inria
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

use super::ProgressLog;

/// How many updates [`BufferedProgressLogger`] accumulates by default before sending
/// them to the underlying [`ProgressLog`].
pub const DEFAULT_THRESHOLD: u16 = 32768;

#[derive(Debug)]
/// A wrapper for `Arc<Mutex<ProgressLogConfig>>` that buffers writes to the underlying
/// [`ProgressLogConfig`] to avoid taking the lock too often.
///
/// This is useful when writing to a common [`ProgressLogConfig`] from multiple threads.
/// # Examples
///
///
/// ```rust
/// use dsi_progress_logger::prelude::*;
///
/// let mut pl = progress_logger!(item_name="pumpkin", display_memory=true);
/// ```
pub struct BufferedProgressLogger<T: DerefMut<Target: ProgressLog>> {
    inner: Arc<Mutex<T>>,
    count: u16,
    threshold: u16,
}

impl<T: DerefMut<Target: ProgressLog>> BufferedProgressLogger<T> {
    pub fn new(pl: Arc<Mutex<T>>) -> Self {
        Self::with_threshold(pl, DEFAULT_THRESHOLD)
    }

    /// Creates a new `BufferedProgressLogger` that accumulates updates up to `threshold`,
    /// before flushing to the underlying [`ProgressLog`].
    pub fn with_threshold(pl: Arc<Mutex<T>>, threshold: u16) -> Self {
        Self {
            inner: pl,
            count: 0,
            threshold,
        }
    }

    pub fn inner(&self) -> &Arc<Mutex<T>> {
        &self.inner
    }

    pub fn flush(&mut self) {
        if self.count > 0 {
            self.inner
                .lock()
                .unwrap()
                .update_with_count(self.count.into());
            self.count = 0;
        }
    }
}

impl<T: DerefMut<Target: ProgressLog>> ProgressLog for BufferedProgressLogger<T> {
    #[inline]
    fn update(&mut self) {
        self.update_with_count(1)
    }
    #[inline]
    fn update_with_count(&mut self, count: usize) {
        match usize::from(self.count).checked_add(count) {
            None => {
                // Sum overflows, update in two steps
                let mut inner = self.inner.lock().unwrap();
                inner.update_with_count(self.count.into());
                inner.update_with_count(count);
                self.count = 0;
            }
            Some(total_count) => {
                if total_count >= usize::from(self.threshold) {
                    // Threshold reached, time to flush to the inner ProgressLogConfig
                    let mut inner = self.inner.lock().unwrap();
                    inner.update_with_count(total_count);
                    self.count = 0;
                } else {
                    // total_count is lower than self.threshold, which is a u16;
                    // so total_count fits in u16.
                    self.count = total_count as u16;
                }
            }
        }
    }
    #[inline]
    fn light_update(&mut self) {
        self.update_with_count(1)
    }
    #[inline]
    fn update_and_display(&mut self) {
        let mut inner = self.inner.lock().unwrap();
        inner.update_with_count(self.count.into());
        self.count = 0;
        inner.update_and_display()
    }
}

/// Flush count buffer to the inner [`ProgressLog`]
impl<T: DerefMut<Target: ProgressLog>> Drop for BufferedProgressLogger<T> {
    fn drop(&mut self) {
        if self.count > 0 {
            self.inner
                .lock()
                .unwrap()
                .update_with_count(self.count.into());
            self.count = 0;
        }
    }
}

impl<T: DerefMut<Target: ProgressLog>> Clone for BufferedProgressLogger<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            count: 0, // Not copied or it would cause double-counting!
            threshold: self.threshold,
        }
    }
}
