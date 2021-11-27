# Current Development Activities

* End handling is incomplete- should append a keying end frame, this could overflow but does not require a WPM/Polarity
  frame since it’s not actual keying that needs decoding wrt a WPM. An end keying event should automatically cause an
  emit after it is encoded.

# Known problems
* Generate the proper configured sidetone sine wave; will have to have a lock around 
it so do a double buffered swap of the generated wave on frequency change.

* Sidetone output via bluetooth headphones has appalling latency - investigate whether dropping the output sample 
  rate to 8000Hz would improve matters.
