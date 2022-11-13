/*
 * The Channel Encoder has a similar design to that in FT8/WSJT-X. The size of the input data is
 * different, as is the LDPC matrix dimensions. The CRC is identical to FT8, as its number of bits
 * is not critical to error control; the LDPC performance dominates. See discussions on the WSJT-X
 * mailing list.
 * The Channel Encoder receives SourceEncodings on its input bus, applies the CRC, then the LDPC
 * then the resulting data is split into 4-bit fields (each can hold a number from 0-15 hence the
 * 16 tones used), these are then mapped to a set of 4-bit Gray codes. A ramping-up symbol is
 * emitted followed by a 7x7 Costas Array, then the encoded 4-bit symbols, followed by a final
 * ramping-down symbol.
 * Ramping symbols have duration 20ms; 4-bit tone symbols have duration 160ms.
 * The transmitter will then output a Gaussian Frequency Shift Keyed tone for each (either by
 * generating tones starting at the currently configured transmit audio offset, or by directly
 * controlling a DDS chip). Tones are spaced 6.25Hz apart, same as FT8, yielding a 100Hz bandwidth.
 *
 * See https://wsjtx.groups.io/g/main/topic/ft8_and_fst4_crc_differences/82267784?p=,,,20,0,0,0::recentpostdate%2Fsticky,,,20,2,0,82267784
 * from Steve K9AN: "there is no reason to calculate the CRC first and then encode. You can cascade
 * the CRC generator matrix with the LDPC code generator matrix once and then use a single
 * vector-matrix multiply to calculate all of the CRC+parity bits. This approach eliminates the need
 * for a separate CRC calculation and would save you a little bit of memory as well"
 */


// why did ft8 choose a 14 bit CRC? They upgraded from 12 to 14 with version 2.0. It doesn't seem
// critical, as the LDPC always returns valid codewords. The CRC just allows the wrong codeword to
// be corrected.
// source_encodinp.rs SOURCE_ENCODER_BLOCK_SIZE_IN_BITS is currently 112.


// geometry of the LDPC (X, Y) where Y is source encoder frame length in bits + CRC length.
// X is Y+number of parity bits.
// X must be divisible by 3, as we use 3 bits per tone. There will be X/3 channel symbols.


// TODO BusInput<SourceEncoding>
// TODO take block: Vec<u8> from the SourceEncoding, CRC it, LDPC it and the CRC, Gray encode it
// then the resulting Vec<Gray> is emitted as a ChannelEncoding. (Gray is a 3-bit quantity in a
// u8). The Transmitter then modulates that.
// TODO BusOutput<ChannelEncoding>


#[cfg(test)]
#[path = "./channel_encoder_spec.rs"]
mod channel_encoder_spec;
