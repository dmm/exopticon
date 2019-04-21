use rmp::encode::{write_map_len, write_str, ValueWriteError};
use rmp::Marker;
use rmp_serde::encode::VariantWriter;
use std::io::Write;

/// An empty struct to implement `VariantWriter` so we can serialize frames as structs/maps.
pub struct StructMapWriter;

impl VariantWriter for StructMapWriter {
    fn write_struct_len<W: Write>(&self, wr: &mut W, len: u32) -> Result<Marker, ValueWriteError> {
        write_map_len(wr, len)
    }

    fn write_field_name<W: Write>(&self, wr: &mut W, key: &str) -> Result<(), ValueWriteError> {
        write_str(wr, key)
    }
}
