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


// TODO BusInput<SourceEncoding>
// TODO take block: Vec<u8> from the SourceEncoding, CRC it, LDPC it and the CRC, Gray encode it
// then the resulting Vec<Gray> is emitted as a ChannelEncoding. (Gray is a 3-bit quantity in a
// u8). The Transmitter then modulates that.
// TODO BusOutput<ChannelEncoding>


use log::debug;
use metered::time_source::{Instant, StdInstant};
use crate::libs::channel_codec::channel_encoding::{ChannelEncoding, ChannelSymbol};
use crate::libs::channel_codec::crc::crc14;
use crate::libs::channel_codec::ldpc::{encode_packed_message, pack_message};
use crate::libs::source_codec::source_encoding::SourceEncoding;
use crate::libs::transform_bus::transform_bus::TransformBus;
use pretty_hex::*;
use crate::libs::channel_codec::gray::to_gray_code;

/*
#[readonly::make]
pub struct ChannelEncoder {
    terminate: Arc<AtomicBool>,
    // Shared between thread and ChannelEncoder
    input_rx: Arc<Mutex<Option<Arc<Mutex<BusReader<SourceEncoding>>>>>>,
    // Shared between thread and ChannelEncoder and ChannelEncoderShared
    output_tx: Arc<Mutex<Option<Arc<Mutex<Bus<ChannelEncoding>>>>>>,
    // storage: Arc<RwLock<Box<dyn SourceEncodingBuilder + Send + Sync>>>, // ?? Is it Send + Sync?
    // Send + Sync are here so the DefaultSourceEncoder can be stored in an rstest fixture that
    // is moved into a panic_after test's thread.
    // thread_handle: Mutex<Option<JoinHandle<()>>>,
    // shared: Arc<Mutex<SourceEncoderShared>>,
    // block_size_in_bits: usize,
}
*/

pub fn source_encoding_to_channel_encoding(source_encoding: SourceEncoding) -> ChannelEncoding {
    let encode_duration = StdInstant::now();
    let crc = crc14(&source_encoding.block.as_slice());
    let packed_message = pack_message(&source_encoding.block, false, false, crc);
    let code_word = encode_packed_message(&packed_message);

    let hexdump = pretty_hex(&code_word.as_slice());
    let hexdump_lines = hexdump.split("\n");
    for line in hexdump_lines {
        debug!("Code word {}", line);
    }

    // Now convert the code_word into a Vec<ChannelSymbol>
    let mut channel_symbols: Vec<ChannelSymbol> = Vec::new();

    // TODO interleave nybbles of the codeword? Does fading more adversely affect different parts of
    // the code word - e.g. if the LDPC data is damaged, does that make recovery harder than if
    // the source data is damaged?

    // TODO Costas Array - possibly just use the same 7x7 array as FT8, using tones 0-6?

    // Convert each nybble of the codeword into its Gray code.
    for byte in code_word {
        channel_symbols.push(to_gray_code(byte >> 4) as ChannelSymbol );
        channel_symbols.push(to_gray_code(byte & 0x0f) as ChannelSymbol )
    }

    let channel_symbols_len = channel_symbols.len();
    let out = ChannelEncoding { block: channel_symbols, is_end: source_encoding.is_end };
    debug!("Channel encoding done in {}ms; {} symbols", encode_duration.elapsed_time(), channel_symbols_len);
    return out;
}


pub type ChannelEncoder = TransformBus<SourceEncoding, ChannelEncoding>;

// Multiple traits: using supertraits and a blanket implementation ... thanks to
// https://tousu.in/qa/?qa=424751/
/*
pub trait ChannelEncoderTrait: BusInput<SourceEncoding> + BusOutput<ChannelEncoding> {}
impl<T: BusInput<SourceEncoding> + BusOutput<ChannelEncoding>> ChannelEncoderTrait for T {}

impl BusInput<SourceEncoding> for ChannelEncoder {
    fn clear_input_rx(&mut self) {
        match self.input_rx.lock() {
            Ok(mut locked) => { *locked = None; }
            Err(_) => {}
        }
    }

    fn set_input_rx(&mut self, input_rx: Arc<Mutex<BusReader<SourceEncoding>>>) {
        match self.input_rx.lock() {
            Ok(mut locked) => { *locked = Some(input_rx); }
            Err(_) => {}
        }
    }
}

impl BusOutput<ChannelEncoding> for ChannelEncoder {
    fn clear_output_tx(&mut self) {
        match self.output_tx.lock() {
            Ok(mut locked) => {
                *locked = None;
            }
            Err(_) => {}
        }
    }

    fn set_output_tx(&mut self, output_tx: Arc<Mutex<Bus<ChannelEncoding>>>) {
        match self.output_tx.lock() {
            Ok(mut locked) => { *locked = Some(output_tx); }
            Err(_) => {}
        }
    }
}

impl ChannelEncoder {
    pub fn new(terminate: Arc<AtomicBool>) -> Self {
        let arc_terminate = terminate.clone();

        // Share this holder between the ChannelEncoder and its thread
        let input_rx_holder: Arc<Mutex<Option<Arc<Mutex<BusReader<SourceEncoding>>>>>> = Arc::new(Mutex::new(None));
        let move_clone_input_rx_holder = input_rx_holder.clone();

        // Share this holder between the ChannelEncoder and the ChannelEncoderShared
        let output_tx_holder: Arc<Mutex<Option<Arc<Mutex<Bus<ChannelEncoding>>>>>> = Arc::new(Mutex::new(None));
        let move_clone_output_tx_holder = output_tx_holder.clone();

        let shared = Mutex::new(ChannelEncoderShared {
            storage: arc_storage.clone(),
            keying_encoder: encoder,
            source_encoder_tx: move_clone_output_tx_holder,
            is_mark: true,
            sent_wpm_polarity: false,
            keying_speed: 0,
        });
        let arc_shared = Arc::new(shared);
        let arc_shared_cloned = arc_shared.clone();
        let thread_handle = thread::spawn(move || {
            let mut keyer_thread = SourceEncoderKeyerThread::new(move_clone_input_rx_holder,
                                                                 arc_terminate,
                                                                 arc_shared.clone());
            keyer_thread.thread_runner();
        });

        Self {
            keyer_speed: 12,
            terminate,
            input_rx: input_rx_holder,    // Modified by BusInput
            output_tx: output_tx_holder,  // Modified by BusOutput
            storage: arc_storage_cloned,
            thread_handle: Mutex::new(Some(thread_handle)),
            shared: arc_shared_cloned,
            block_size_in_bits
        }

}
*/

#[cfg(test)]
#[path = "./channel_encoder_spec.rs"]
mod channel_encoder_spec;
