# Current Development Activities

* Main: wiring up the channel encoder and transmitter.

Next up for research:
* Costas array: is there an escaping mechanism, such that the Costas array does not occur in the binary output of the
  channel encoder?
* Costas array: devise one suitable for 16 tones.

Other refactorings to do:
* Application wiring:
  * The diag_application_spec.rs needs to have the 'source encoder diag' code moved here, out of the main
    program, and the main program command line handling should have it removed - such 'diags' are now implemented
    as tests.
* Playback improvements:
  * Split the play method into a scan through the frame to extract timings of keyings into a Vec, and then a pass 
   through that list to schedule the tone generations. Separate the use of a ToneGenerator/ScheduledThreadPool from the
   conversion of source encoding to timings - would permit mocks to sense the allocation/deallocation of channels, and
   stubs to collect the timing information.
  * Playback gap delay - need to work out optimal delay for first frames. Could be based on WPM, and whether there are
   metadata frames in a block. Create many dummy QSO texts, send them through the Playback at varying WPM from 5-60, and
   determine how much delay is needed so that no gaps are present. Use this to seed the optimal first frame gap delay.

Considerations:
* Transmitter / GFSK Modulation: choose suitable number of tones for slow/fast (narrow/wide) modulations.



# Known problems
* Sidetone output via bluetooth headphones has appalling latency - investigate whether dropping the output sample
  rate to 8000Hz would improve matters.

* Need to upgrade to rust edition 2021 - doing so causes test compilation failure in arduino_keyer_io_spec but this must
  be corrected.

* Checking for serial port existence in main - needs implementing correctly for Windows.

* There is currently no metadata encoding/decoding.

* Source encoder KeyerEnd handling is incomplete- should append a keying end frame, this could overflow but does not
  require a WPM/Polarity frame since it’s not actual keying that needs decoding wrt a WPM. An end keying event should
  automatically cause an emit after it is encoded.
