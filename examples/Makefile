SRCS := $(wildcard *.tny)
ASMS := $(SRCS:.tny=.asm)
BINS := $(SRCS:.tny=.bin)

all: $(BINS)

%.asm: %.tny
	teenyc $< -o $@

%.bin: %.asm
	tnasm $<

clean:
	rm -f $(BINS) $(ASMS)

.PHONY: all clean
