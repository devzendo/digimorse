# Current Development Activities

* Back-to-back Source Encoder diagnostic mode - keying is encoded into frames, placed into blocks, then sent to a
  delay bus, after a short delay, the blocks are emitted, decoded into frames, and their keying played back.

* Application wiring.

* Playback improvements:
** Split the play method into a scan through the frame to extract timings of keyings into a Vec, and then a pass 
   through that list to schedule the tone generations. Separate the use of a ToneGenerator/ScheduledThreadPool from the
   conversion of source encoding to timings - would permit mocks to sense the allocation/deallocation of channels, and
   stubs to collect the timing information.
** Playback gap delay - need to work out optimal delay for first frames. Could be based on WPM, and whether there are
   metadata frames in a block. Create many dummy QSO texts, send them through the Playback at varying WPM from 5-60, and
   determine how much delay is needed so that no gaps are present. Use this to seed the optimal first frame gap delay.

* End handling is incomplete- should append a keying end frame, this could overflow but does not require a WPM/Polarity
  frame since it’s not actual keying that needs decoding wrt a WPM. An end keying event should automatically cause an
  emit after it is encoded.


# Known problems
* Tone generation has a faint artifact. Is this due to the waveform, should be able to regenerate it as floats?

* Sidetone output via bluetooth headphones has appalling latency - investigate whether dropping the output sample 
  rate to 8000Hz would improve matters.

* Checking for serial port existence in main - needs implementing correctly for Windows.

* There is currently no metadata encoding/decoding.
