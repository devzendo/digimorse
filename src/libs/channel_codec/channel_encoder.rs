/*
 * The Channel Encoder has a similar design to that in FT8/WSJT-X. The size of the input data is
 * different, as are the CRC and LDPC matrix dimensions.
 * The Channel Encoder receives SourceEncodings on its input bus, applies the CRC, then the LDPC
 * then the resulting data is split into 3-bit fields (each can hold a number from 0-7 hence the
 * 8 tones used), these are then mapped to a set of 3-bit Gray codes. A ramping-up symbol is emitted
 * followed by a 7x7 Costas Array, then the encoded 3-bit symbols, followed by a final ramping-down
 * symbol.
 * Ramping symbols have duration 20ms; 3-bit tone symbols have duration 160ms.
 * The transmitter will then output a Gaussian Frequency Shift Keyed tone for each (either by
 * generating tones starting at the currently configured transmit audio offset, or by directly
 * controlling a DDS chip). Tones are spaced 6.25Hz apart, same as FT8, yielding a 50Hz bandwidth.
 *
 * See https://wsjtx.groups.io/g/main/topic/ft8_and_fst4_crc_differences/82267784?p=,,,20,0,0,0::recentpostdate%2Fsticky,,,20,2,0,82267784
 * from Steve K9AN: "there is no reason to calculate the CRC first and then encode. You can cascade
 * the CRC generator matrix with the LDPC code generator matrix once and then use a single
 * vector-matrix multiply to calculate all of the CRC+parity bits. This approach eliminates the need
 * for a separate CRC calculation and would save you a little bit of memory as well"
 */


// why did ft8 choose a 14 bit CRC?
// source_encodinp.rs SOURCE_ENCODER_BLOCK_SIZE_IN_BITS is currently 128.


// geometry of the LDPC (X, Y) where Y is source encoder frame length in bits + CRC length.
// X is Y+number of parity bits.
// X must be divisible by 3, as we use 3 bits per tone. There will be X/3 channel symbols.


#[cfg(test)]
#[path = "./channel_encoder_spec.rs"]
mod channel_encoder_spec;
