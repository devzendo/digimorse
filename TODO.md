# Current Development Activities


Next developments:
* Transmitter: Fix showstoppers!
  * Transmitter audio is clunky - every modulated channel encoding is starting with a ramp up
* GUI: Ensuring the operation of the GUI indicators from the rest of the system.
* Application: when the keyer speed is set on the application, set it on any configured source encoder, as well as the keyer.
* Receiver - callback receiving audio from the radio's speaker (the microphone PortAudio device).
* Receiver - downsample the incoming audio. Do we need a pool of outgoing audio buffers?
* Receiver - allow callback audio to be overridden by an input .wav file, by reading the whole waveform into memory, and
  overwriting the callback audio buffer.
* Application: Allow the Receiver to be wired in, with a ReceivedWaveformBus as output.
* Decoder: Listens to the ReceivedWaveformBus, FFTs it, quantizes that for the UI, sends to the GUIInput. Gives the relevant subset
  of data to each StationDecoder in parallel. Performs Costas Array detection across the spectrum; adds StationDecoders if new
  array found.
* Add a ListKeyerDevices mode?
* Log the current keyer device/port on startup, if used.
* GUI: Trap Cmd-Q/Alt-F4 for shutdown.

Next up for research:
* Costas array: is there an escaping mechanism, such that the Costas array does not occur in the binary output of the
  channel encoder?
* Costas array: devise one suitable for 16 tones.

Other refactorings to do:
* Text-to-Morse conversion does not handle prosigns entered as `<KN>` or just `KN` in upper case. There are my shortcuts
  though: + for AR, | for SK, = for BT, > for KN.
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
* Transmitter/Modulator: Make it a struct so the GFSK pulse can be computed once.

Considerations:
* Transmitter / GFSK Modulation: choose suitable number of tones for slow/fast (narrow/wide) modulations.
* When using the command line config set options, should the application continue to start up after setting?
  It currently does, which is a bit un-nerving!



# Known problems
* Showstopper: Only the first modulation of some keying comes out of the speaker. Doesn't matter if it's from key/GUI.

* Bug: GUI does not terminate on terminal Ctrl-C

* Sidetone output via bluetooth headphones has appalling latency - investigate whether dropping the output sample
  rate to 8000Hz would improve matters.

* Need to upgrade to rust edition 2021 - doing so causes test compilation failure in arduino_keyer_io_spec but this must
  be corrected.

* Checking for serial port existence in main - needs implementing correctly for Windows.

* There is currently no metadata encoding/decoding.

* Source encoder KeyerEnd handling is incomplete- should append a keying end frame, this could overflow but does not
  require a WPM/Polarity frame since it’s not actual keying that needs decoding wrt a WPM. An end keying event should
  automatically cause an emit after it is encoded.
