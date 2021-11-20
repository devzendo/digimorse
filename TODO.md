# Current Development Activities

* Use the bitvec_source_encoding_builder in the source_encoder to encode keying events.

# Known problems
* Generate the proper configured sidetone sine wave; will have to have a lock around 
it so do a double buffered swap of the generated wave on frequency change.

* Sidetone output via bluetooth headphones has appalling latency - investigate whether dropping the output sample 
  rate to 8000Hz would improve matters.
