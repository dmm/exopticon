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

use gst::prelude::*;

fn worker_main() {
    gst::init?;

    let args: Vec<_> = env::args().collect();
    let uri: &str = if args.len() == 3 {
        args[3].as_ref()
    } else {
        println!("Usage: exopticon --snapshot <file_path>");
        std::process::exit(-1)
    };

    let pipeline = gst::Pipelined::new(None);
    let src = gst::ElementFactory::make("uridecodebin", None)
        .map_err(|_| MissingElement("uridecodebin"))?;
    src.set_property("uri", &uri)?;

    pipeline.add_many(&[&src, &decodebin])?;
    gst::Element::link_many(&[&src, &decodebin])?;

    let pipeline_weak = pipeline.downgrade();

    decodebin.connect_pad_added(move |dbin, src_pad| {});
}
