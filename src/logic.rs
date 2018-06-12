//! Lazy logical operators on bit slices.

use Bits;
use BlockType;

use std::cmp::{max, min};

/// Extension trait for bitwise logic operations on bit vectors.
///
/// The operations return lazy bit vectors, which can be forced and copied
/// into strict bit vectors with `BitVec::from_bits`.
pub trait BitsLogic: Bits {
    /// Returns an object that inverts the values of all the bits in `self`.
    fn bits_not(self) -> BitsNot<Self>
        where Self: Sized
    {
        BitsNot(self)
    }

    /// Returns an object that lazily computes the bit-wise conjunction
    /// of two bit-vector-likes.
    fn bits_and<Other>(self, other: Other) -> BitsAnd<Self, Other>
        where Self: Sized,
              Other: Bits<Block = Self::Block> {

        BitsAnd(BitsBinOp::new(self, other))
    }

    /// Returns an object that lazily computes the bit-wise disjunction
    /// of two bit-vector-likes.
    fn bits_or<Other>(self, other: Other) -> BitsOr<Self, Other>
        where Self: Sized,
              Other: Bits<Block = Self::Block> {

        BitsOr(BitsBinOp::new(self, other))
    }

    /// Returns an object that lazily computes the bit-wise xor of two
    /// bit-vector-likes.
    fn bits_xor<Other>(self, other: Other) -> BitsXor<Self, Other>
        where Self: Sized,
              Other: Bits<Block = Self::Block> {

        BitsXor(BitsBinOp::new(self, other))
    }
}

impl<T: Bits> BitsLogic for T {}

/// The result of [`BitsLogic::bits_not`](trait.BitsLogic.html#method.bits_not),
/// which inverts the bits of a bit vector.
pub struct BitsNot<T>(T);

/// The result of [`BitsLogic::bits_and`](trait.BitsLogic.html#method.bits_and)
/// on types that implement `Bits`.
pub struct BitsAnd<T, U>(BitsBinOp<T, U>);

/// The result of [`BitsLogic::bits_or`](trait.BitsLogic.html#method.bits_or)
/// on types that implement `Bits`.
pub struct BitsOr<T, U>(BitsBinOp<T, U>);

/// The result of [`BitsLogic::bits_xor`](trait.BitsLogic.html#method.bits_xor)
/// on types that implement `Bits`.
pub struct BitsXor<T, U>(BitsBinOp<T, U>);

/// Used to store the two operands to a bitwise logical operation on
/// `Bits`es, along with the length of the result (max the length of
/// the operands) and the offset of the result (see invariant below).
/// (Note that both `len` and `off` are derivable from `op1` and `op2`,
/// but it probably makes sense to cache them.)
struct BitsBinOp<T, U> {
    op1: T,
    op2: U,
    len: u64,
    off: u8,
}
// Invariant:
//  - off != 0  ==>  off == op1.bit_offset() == op2.bit_offset()

// Precondition: `offset == bits.bit_offset() || offset == 0`.
fn get_unaligned_block<T: Bits>(bits: &T, offset: u8, position: usize) -> T::Block {
    if offset == bits.bit_offset() {
        if position < bits.block_len() {
            bits.get_block(position)
        } else {
            T::Block::zero()
        }
    } else {
        debug_assert_eq!( offset, 0 );
        let start = T::Block::mul_nbits(position);
        if start < bits.bit_len() {
            let nbits = min(T::Block::nbits() as u64, bits.bit_len() - start);
            bits.get_bits(start, nbits as usize)
        } else {
            T::Block::zero()
        }
    }
}

fn get_overflow_bit<T: Bits>(bits: &T, position: u64) -> bool {
    if position < bits.bit_len() {
        bits.get_bit(position)
    } else {
        false
    }
}

impl<T: Bits, U: Bits<Block = T::Block>> BitsBinOp<T, U> {
    fn new(op1: T, op2: U) -> Self {
        let len = max(op1.bit_len(), op2.bit_len());

        let op1_off = op1.bit_offset();
        let op2_off = op2.bit_offset();
        let off = if op1_off == op2_off {op1_off} else {0};

        BitsBinOp { op1, op2, len, off }
    }

    fn bit1(&self, position: u64) -> bool {
        get_overflow_bit(&self.op1, position)
    }

    fn bit2(&self, position: u64) -> bool {
        get_overflow_bit(&self.op2, position)
    }

    fn block1(&self, position: usize) -> T::Block {
        get_unaligned_block(&self.op1, self.off, position)
    }

    fn block2(&self, position: usize) -> T::Block {
        get_unaligned_block(&self.op2, self.off, position)
    }
}

impl<T: Bits> Bits for BitsNot<T> {
    type Block = T::Block;

    fn bit_len(&self) -> u64 {
        self.0.bit_len()
    }

    fn bit_offset(&self) -> u8 {
        self.0.bit_offset()
    }

    fn get_bit(&self, position: u64) -> bool {
        !self.0.get_bit(position)
    }

    fn get_block(&self, position: usize) -> Self::Block {
        !self.0.get_block(position)
    }
}

impl<T, U> Bits for BitsAnd<T, U>
    where T: Bits,
          U: Bits<Block = T::Block>
{
    type Block = T::Block;

    fn bit_len(&self) -> u64 {
        self.0.len
    }

    fn bit_offset(&self) -> u8 {
        self.0.off
    }

    fn get_bit(&self, position: u64) -> bool {
        self.0.bit1(position) & self.0.bit2(position)
    }

    fn get_block(&self, position: usize) -> Self::Block {
        self.0.block1(position) & self.0.block2(position)
    }
}

impl<T, U> Bits for BitsOr<T, U>
    where T: Bits,
          U: Bits<Block = T::Block>
{
    type Block = T::Block;

    fn bit_len(&self) -> u64 {
        self.0.len
    }

    fn bit_offset(&self) -> u8 {
        self.0.off
    }

    fn get_bit(&self, position: u64) -> bool {
        self.0.bit1(position) | self.0.bit2(position)
    }

    fn get_block(&self, position: usize) -> Self::Block {
        self.0.block1(position) | self.0.block2(position)
    }
}

impl<T, U> Bits for BitsXor<T, U>
    where T: Bits,
          U: Bits<Block = T::Block>
{
    type Block = T::Block;

    fn bit_len(&self) -> u64 {
        self.0.len
    }

    fn bit_offset(&self) -> u8 {
        self.0.off
    }

    fn get_bit(&self, position: u64) -> bool {
        self.0.bit1(position) ^ self.0.bit2(position)
    }

    fn get_block(&self, position: usize) -> Self::Block {
        self.0.block1(position) ^ self.0.block2(position)
    }
}

#[cfg(test)]
mod test {
    use {Bits, BitVec, BitSliceable};
    use super::BitsLogic;

    fn assert_0001<T: Bits>(bits: &T) {
        assert_eq!( bits.bit_len(), 4 );

        assert!( !bits.get_bit(0) );
        assert!( !bits.get_bit(1) );
        assert!( !bits.get_bit(2) );
        assert!(  bits.get_bit(3) );

        let bv = BitVec::from_bits(bits);
        assert_eq!( bv, bit_vec![false, false, false, true] );
    }

    #[test]
    fn simple_not() {
        let bv: BitVec = bit_vec![true, true, true, false,];
        let not_bits = (&bv).bits_not();
        assert_0001(&not_bits);
    }

    #[test]
    fn simple_and() {
        let bv1: BitVec<u8> = bit_vec![ false, false, true, true, ];
        let bv2: BitVec<u8> = bit_vec![ false, true, false, true, ];
        let and_bits = (&bv1).bits_and(&bv2);
        assert_0001(&and_bits);
    }

    #[test]
    fn and_with_same_offset() {
        let bv1: BitVec<u8> = bit_vec![ true, false, false, true, true ];
        let bv2: BitVec<u8> = bit_vec![ true, false, true, false, true ];
        let bv_slice1 = bv1.bit_slice(1..);
        let bv_slice2 = bv2.bit_slice(1..);
        let and_bits = (&bv_slice1).bits_and(&bv_slice2);
        assert_0001(&and_bits);
    }

    #[test]
    fn and_with_different_offset() {
        let bv1: BitVec<u8> = bit_vec![ true, true, false, false, true, true ];
        let bv2: BitVec<u8> = bit_vec![ true, false, true, false, true ];
        let bv_slice1 = bv1.bit_slice(2..);
        let bv_slice2 = bv2.bit_slice(1..);
        let and_bits = (&bv_slice1).bits_and(&bv_slice2);
        assert_0001(&and_bits);
    }
}