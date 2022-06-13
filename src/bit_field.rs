// A poor man's bit_field.

use core::ops::Bound;
use core::ops::RangeBounds;

pub trait BitField: Sized {
    fn set_bits<R: RangeBounds<u8>>(&mut self, range: R, bits: Self);
}

// This impl depends on the size of u16. We must be careful not to
// overflow when migrate it to other integral types.
impl BitField for u16 {
    /// Set self's bit pattern in range to bits.
    /// # Panics
    /// Panics if the range isn't valid or bits excess the range.
    fn set_bits<R: RangeBounds<u8>>(&mut self, range: R, bits: u16) {
        const INVALID_BIT_RANGE: &str = "invalid bit range";

        let start = match range.start_bound() {
            Bound::Included(&i) => i,
            Bound::Excluded(&i) => i.checked_add(1).expect(INVALID_BIT_RANGE),
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(&i) => i,
            Bound::Excluded(&i) => i.checked_sub(1).expect(INVALID_BIT_RANGE),
            Bound::Unbounded => (u16::BITS - 1) as u8,
        };
        assert!(
            start <= end && end < u16::BITS as u8,
            "{}",
            INVALID_BIT_RANGE
        );
        // Get a full mask for the range span.
        let mask: u16 = ((1u32 << (end - start + 1)) - 1) as u16;
        assert!(bits & !mask == 0, "bits fall outside of range");
        // Clear that range and put bits in.
        *self = (*self & !(mask << start)) | bits << start;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_bit_field_basic() {
        let mut bits: u16 = 0;
        bits.set_bits(.., 0b111);
        assert_eq!(bits, 0b111);
        bits.set_bits(..=3, 0);
        assert_eq!(bits, 0);
        bits.set_bits(3..=5, 0b101);
        assert_eq!(bits, 0b101000);
        bits.set_bits(4..5, 1);
        assert_eq!(bits, 0b111000);
        bits.set_bits(4.., 0);
        assert_eq!(bits, 0b1000);
    }
}
