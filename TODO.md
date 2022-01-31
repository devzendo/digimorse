# Current Development Activities

* Back-to-back Source Encoder diagnostic mode - keying is encoded into frames, placed into blocks, then sent to a
  delay bus, after a short delay, the blocks are emitted, decoded into frames, and their keying played back.

* ToneGenerator needs deallocate_channel writing. How to map callsign hash / audio offset (StationDetails) to these
  offsets? 
* Playback needs to allocate channels via the ToneGenerator.

* End handling is incomplete- should append a keying end frame, this could overflow but does not require a WPM/Polarity
  frame since it’s not actual keying that needs decoding wrt a WPM. An end keying event should automatically cause an
  emit after it is encoded.

# Known problems
* Tone generation has a faint artifact.

* Sidetone output via bluetooth headphones has appalling latency - investigate whether dropping the output sample 
  rate to 8000Hz would improve matters.
