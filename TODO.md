# Current Development Activities

* Back-to-back Source Encoder diagnostic mode - keying is encoded into frames, placed into blocks, then sent to a
  delay bus, after a short delay, the blocks are emitted, decoded into frames, and their keying played back.

* ArduinoKeyer only needs a single channel to communicate commands and output-bus-setting between main and thread.
* Use rstest for ArduinoKeyer test fixtures.

* End handling is incomplete- should append a keying end frame, this could overflow but does not require a WPM/Polarity
  frame since it’s not actual keying that needs decoding wrt a WPM. An end keying event should automatically cause an
  emit after it is encoded.

# Known problems
* Polarity inversion fault in SourceEncoderDiag.

* Tone generation has a faint artifact. Is this due to the waveform, should be able to regenerate it as floats?

* Sidetone output via bluetooth headphones has appalling latency - investigate whether dropping the output sample 
  rate to 8000Hz would improve matters.

* Checking for serial port existence in main - needs implementing correctly for Windows.
* 