# Current Development Activities

* Back-to-back Source Encoder diagnostic mode - keying is encoded into frames, placed into blocks, then sent to a
  delay bus, after a short delay, the blocks are emitted, decoded into frames, and their keying played back.

* End handling is incomplete- should append a keying end frame, this could overflow but does not require a WPM/Polarity
  frame since it’s not actual keying that needs decoding wrt a WPM. An end keying event should automatically cause an
  emit after it is encoded.

# Known problems
* Fault in SourceEncoderDiag:
  thread '<unnamed>' panicked at 'No speed has been set on the DefaultKeyingEncoder', src/libs/source_codec/keying_encoder.rs:97:13
  stack backtrace:
  0: std::panicking::begin_panic
  1: <digimorse::libs::source_codec::keying_encoder::DefaultKeyingEncoder as digimorse::libs::source_codec::keying_encoder::KeyingEncoder>::encode_keying
  2: digimorse::libs::source_codec::source_encoder::SourceEncoderKeyerThread::thread_runner
* 
* Tone generation has a faint artifact. Is this due to the waveform, should be able to regenerate it as floats?

* Sidetone output via bluetooth headphones has appalling latency - investigate whether dropping the output sample 
  rate to 8000Hz would improve matters.

* Checking for serial port existence in main - needs implementing correctly for Windows.
* 