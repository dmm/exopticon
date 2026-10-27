/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2021 David Matthew Mattli <dmm@mattli.us>
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

use std::{hint::black_box, time::Duration};

use criterion::{Criterion, Throughput, criterion_group, criterion_main};

use exserial::models::{CaptureMessage, PacketEncoding};

extern crate exserial;

pub fn criterion_benchmark(c: &mut Criterion) {
    // make a vec of about 318KiB to represent a compressed video packet
    let mut data: Vec<u8> = Vec::with_capacity(321900);
    // initialize with random values
    for _ in 0..data.capacity() {
        data.push(rand::random());
    }

    let frame = CaptureMessage::Packet {
        encoding: PacketEncoding::H264,
        data,
        timestamp: 90_000,
        duration: 33_333,
    };

    let serialized = postcard::to_allocvec(&frame).expect("Unable to serialize message!");

    let mut group = c.benchmark_group("serialize deserialize throughput");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(20));
    group.bench_function("postcard serialize packet", |b| {
        b.iter(|| {
            postcard::to_allocvec(black_box(&frame)).expect("Unable to serialize message!");
        });
    });

    group.bench_function("postcard deserialize packet", |b| {
        b.iter(|| {
            let _frame: CaptureMessage = postcard::from_bytes(black_box(&serialized))
                .expect("Unable to deserialize message!");
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
