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
