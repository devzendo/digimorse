# Current Development Activities

* End handling is incomplete- should append a keying end frame, this could overflow but does not require a WPM/Polarity
  frame since it’s not actual keying that needs decoding wrt a WPM. An end keying event should automatically cause an
  emit after it is encoded.

# Known problems
* Tone generation has a faint artifact.

* Sidetone output via bluetooth headphones has appalling latency - investigate whether dropping the output sample 
  rate to 8000Hz would improve matters.
