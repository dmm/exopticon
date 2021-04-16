/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2020 David Matthew Mattli <dmm@mattli.us>
 *
 * This file is part of Exopticon.
 *
 * Exopticon is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Exopticon is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Exopticon.  If not, see <http://www.gnu.org/licenses/>.
 */

//! Implements registry for `PrometheusMetrics`

use actix_web_prom::PrometheusMetrics;
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref DBREG: Mutex<Vec<PrometheusMetrics>> = Mutex::new(Vec::new());
}

/// Returns `PrometheusMetrics`
///
/// # Panics
///
/// Panics if called before `set_metrics()`.
///
pub fn get_metrics() -> PrometheusMetrics {
    DBREG
        .lock()
        .expect("PrometheusMetrics: Unable to lock metrics registry for retrieval.")
        .last()
        .expect("PrometheusMetrics: PrometheusMetrics not set!")
        .clone()
}

/// Set `PrometheusMetrics`. Must be called before `get_metrics()`.
pub fn set_metrics(metrics: PrometheusMetrics) {
    DBREG.lock().expect("unable to set metrics!").push(metrics);
}
