# Current Development Activities

I'm currently refactoring the wiring of the various system objects. The main application is complicated 
as it does all the setup, in all of the main 'modes'. So I'm separating out the wiring
of the system into the Application object. All Bus/BusReaders will be encapsulated there. All
systems that read/write from these Bus/BusReaders currently have their connections set statically
on construction. Later the user will be able to reconfigure the Keyer, Audio devices and Transceiver
at runtime - so the Application must permit dynamic rewiring, and the system objects need to change
to be able to be wired in to the Application. This has been done for all bus-connected system objects: they
now implement BusInput/BusOutput, as they are refactored to permit dynamic wiring into the Application.

* Application wiring.
  * The diag_application_spec.rs needs to have the 'source encoder diag' code moved here, out of the main
  program, and the main program command line handling should have it removed - such 'diags' are now implemented
  as tests.
  
  
My current research activities are around the next system object: the channel encoder:

* Adding error detection by computing a CRC over the source encoder output.
* Adding error correction by computing a LDPC over the CRC'd source encoder output.
* Costas array: is there an escaping mechanism, such that the Costas array does not occur in the binary output of the
  channel encoder?
* Modulation: choose suitable number of tones for slow/fast (narrow/wide) modulations.

Other refactorings to do:
* Playback improvements:
  * Split the play method into a scan through the frame to extract timings of keyings into a Vec, and then a pass 
   through that list to schedule the tone generations. Separate the use of a ToneGenerator/ScheduledThreadPool from the
   conversion of source encoding to timings - would permit mocks to sense the allocation/deallocation of channels, and
   stubs to collect the timing information.
  * Playback gap delay - need to work out optimal delay for first frames. Could be based on WPM, and whether there are
   metadata frames in a block. Create many dummy QSO texts, send them through the Playback at varying WPM from 5-60, and
   determine how much delay is needed so that no gaps are present. Use this to seed the optimal first frame gap delay.


# Known problems
* Tone generation has a faint artifact. Is this due to the waveform, should be able to regenerate it as floats?

* Sidetone output via bluetooth headphones has appalling latency - investigate whether dropping the output sample 
  rate to 8000Hz would improve matters.

* Checking for serial port existence in main - needs implementing correctly for Windows.

* There is currently no metadata encoding/decoding.

* Source encoder KeyerEnd handling is incomplete- should append a keying end frame, this could overflow but does not
require a WPM/Polarity frame since it’s not actual keying that needs decoding wrt a WPM. An end keying event should
automatically cause an emit after it is encoded.
