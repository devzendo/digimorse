# Current Development Activities

* Generate the proper configured sidetone sine wave; will have to have a lock around 
it so do a double buffered swap of the generated wave on frequency change.

* Get some real sample keyer data to inform source encoder design - variable length
codes.

* Plot the keyer data as a scatter plot to get a feel for the variance around the 
various Morse element timings, to inform the design of the source encoder.

* Sidetone output via bluetooth headphones has appalling latency - investigate whether dropping the output sample 
  rate to 8000Hz would improve matters.
