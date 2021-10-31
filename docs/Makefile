# Author:
#      Wim Looman
# Copyright:
#      Copyright (c) 2010 Wim Looman
# License:
#      GNU General Public License (see http://www.gnu.org/licenses/gpl-3.0.txt)

## User interface, just set the main filename and it will do everything for you
# If you have any extra code or images included list them in EXTRA_FILES
# This should work as long as you have all the .tex, .sty and .bib files in
# the same folder.
MAINFILE = the-digimorse-communications-protocol
EXTRA_FILES := $(shell echo "images/*") 

## Inner workings
OBJECTS = $(shell echo *.tex)
STYLES = $(shell echo *.sty)
BIB = $(shell echo *.bib)

TEMP2 := $(shell mkdir make 2>/dev/null)

.PHONY: all
all: $(MAINFILE).pdf

$(MAINFILE).pdf: $(EXTRA_FILES)
	pdflatex $(MAINFILE)
	pdflatex $(MAINFILE)

make/%.t: %.tex
	touch $@

make/%.s: %.sty
	touch $@

make/bib: $(BIB)
	latex $(MAINFILE)
	bibtex $(MAINFILE)
	touch $@

.PHONY: clean
clean:
	-rm -f *.aux
	-rm -f *.log
	-rm -f *.toc
	-rm -f *.bbl
	-rm -f *.blg
	-rm -f *.out
	-rm -f make/bib

.PHONY: cleanall
cleanall: clean
	-rm -f *.pdf
	-rm -f *.ps
	-rm -f *.dvi
	-rm -rf ./make