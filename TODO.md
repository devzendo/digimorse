# Current Development Activities

* Generate the proper configured sidetone sine wave; will have to have a lock around 
it so do a double buffered swap of the generated wave on frequency change.

* Get some real sample keyer data to inform source encoder design - variable length
codes.

* Plot the keyer data as a histogram to get a feel for the variance around the 
various Morse element timings, to inform the design of the source encoder.
  * Read the sample-qso-m0cuv.csv into R Studio
    * qso <- read.csv(file = "Documents/IdeaProjects/DevZendo.org/digimorse/docs/sample-qso-m0cuv.csv", header = FALSE)
  * Plot it attractively with ggplot2
    * ggplot(qso, aes(x=V2)) + geom_histogram(binwidth = 1)
  * Determine peaks of the dit/dah/wordgap times - reverse this to determine my WPM
  * Annotate with precise dit/dah/wordgap times from Appendix A
  * Annotate with delta of a sample point away from the precise times?
  * Determine best output file format for use with LaTeX (PDF or EPS for diagrams; JPG or PNG for bitmaps)
  * Embed the graph in the LaTeX doc

* Sidetone output via bluetooth headphones has appalling latency - investigate whether dropping the output sample 
  rate to 8000Hz would improve matters.
