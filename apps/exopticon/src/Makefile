mycflags= -W -Wall -Wextra -Wcast-align -Wpointer-arith -Wsign-compare -Wformat=2 \
          -Wno-format-y2k  -Wmissing-braces -Wparentheses -Wtrigraphs -Wstrict-aliasing=2 \
          -std=c99 -flto
devcflags= -fsanitize=undefined -fsanitize=address

LIBRARIES= -lavformat -lavutil -lavcodec
CC ?= gcc
BINDIR ?= ../lib/exopticon

all: production

production: captureserver playbackserver

development: captureserver.c
	${CC}  -O0 -g $(mycflags) $(devcflags) $(CFLAGS) $(LIBRARIES) captureserver.c mpack.c mpack_frame.c -o ${BINDIR}/captureserver

playbackserver: playbackserver.c mpack_frame.c
	${CC} -O0 -g $(mycflags) $(CFLAGS) $(LIBRARIES) playbackserver.c mpack.c mpack_frame.c -o ${BINDIR}/playbackserver

captureserver: captureserver.c mpack_frame.c
	${CC} -O2 -g $(mycflags) $(CFLAGS) $(LIBRARIES) captureserver.c mpack.c mpack_frame.c -o ${BINDIR}/captureserver

.PHONY: clean
clean:
	-rm -f ${BINDIR}/captureserver ${BINDIR}/playbackserver
