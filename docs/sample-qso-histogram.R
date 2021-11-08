library(ggplot2)
library(hrbrthemes)
hrbrthemes::import_roboto_condensed()
qso <- read.csv(file = "sample-qso-m0cuv.csv", header = FALSE)
qso_main <- subset(qso, V2 < 1100)
ggplot(qso_main, aes(x=V2)) + geom_histogram(binwidth = 2) + theme_ipsum_ps() +
  labs(x="Mark/Space duration (ms)", y="Count", title="Histogram of M0CUV keying a sample QSO",
       subtitle="How closely does actual keying track the ideal dit/dah/wordgap durations?",
       caption="Not quite the rhythm machine you thought, eh, Matt?") +
  geom_vline(xintercept=72, color="red", size=1) +
  geom_vline(xintercept=225, color="red", size=1) +
  geom_vline(xintercept=525, color="green", size=1)

# mode, median, mean of the large spike centre on 72.
# the table suggests for a dit of 72, this is between 16 and 17 WPM, (say 16). This gives a dah of 225 and a wordgap of
# 525.

# lack of a large spike for dah, and nothing at all for wordgap is very odd.

ggsave(
  "sample-qso-m0cuv.eps",
  plot = last_plot(),
  device = "eps",
  path = ".",
  scale = 1,
  units = c("in", "cm", "mm", "px"),
  dpi = 600,
  limitsize = TRUE,
  bg = NULL,
)