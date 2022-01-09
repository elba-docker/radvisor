use crate::util::buffer::{Buffer, BufferLike};
use atoi::FromRadix10Checked;
use csv::ByteRecord;
use itoa::{self, Integer};
use num_traits::ops::saturating::SaturatingAdd;
use std::io::{self, Write};

#[derive(Copy, Clone)]
pub enum LazyQuantity<'a, T: FromRadix10Checked + SaturatingAdd + Integer + Copy> {
    /// Contains a zero quantity (result of no aggregation)
    Zero,
    /// Contains a single quantity in its textual form
    Single(&'a [u8]),
    /// Contains an aggregated quantity in its numeric form
    Aggregate(T),
}

impl<'a, T: FromRadix10Checked + SaturatingAdd + Integer + Copy> LazyQuantity<'a, T> {
    /// Adds the given quantity to this one
    pub fn plus<'b: 'a>(self, quantity: &'b [u8]) -> Self {
        match self {
            Self::Zero => Self::Single(quantity),
            Self::Single(current) => {
                match T::from_radix_10_checked(current).0 {
                    // If the conversion failed, downgrade
                    None => Self::Single(quantity),
                    // Otherwise, call the number + number case
                    Some(as_int) => Self::Aggregate(as_int).plus(quantity),
                }
            },
            Self::Aggregate(ref current_int) => match T::from_radix_10_checked(quantity).0 {
                None => self,
                Some(as_int) => Self::Aggregate(current_int.saturating_add(&as_int)),
            },
        }
    }

    /// Writes the quantity into the buffer
    pub fn write_to<const CAP: usize>(self, dest: &mut Buffer<CAP>) -> io::Result<usize> {
        match self {
            Self::Zero => dest.write(b"0"),
            Self::Single(current) => dest.write(current),
            Self::Aggregate(current_int) => {
                let mut itoa_buffer = itoa::Buffer::new();
                let formatted = itoa_buffer.format(current_int);
                dest.write(formatted.as_bytes())
            },
        }
    }

    /// Writes the quantity to a record, using the working buffer as an
    /// intermediate
    pub fn write_to_record<const CAP: usize>(
        self,
        working: &mut Buffer<CAP>,
        record: &mut ByteRecord,
    ) {
        // Write the quantity to to the temporary copy buffer
        working.len = self.write_to(working).unwrap_or(0);

        // Write to the record
        record.push_field(working.content());
        working.clear();
    }
}

impl<'a, T: FromRadix10Checked + SaturatingAdd + Integer + Copy> Default for LazyQuantity<'a, T> {
    fn default() -> Self { Self::Zero }
}
