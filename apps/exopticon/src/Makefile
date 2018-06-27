mycflags= -W -Wall -Wextra -Wcast-align -Wpointer-arith -Wsign-compare -Wformat=2 \
          -Wno-format-y2k  -Wmissing-braces -Wparentheses -Wtrigraphs -Wstrict-aliasing=2 \
          -std=c99 -flto -D_FORTIFY_SOURCE=2 -fstack-protector-strong
devcflags= -fsanitize=undefined -fsanitize=address

LIBRARIES= -lavformat -lavutil -lavcodec -lswscale
CC ?= gcc
EXWORKER_BINDIR ?= ../lib/exopticon

all: ${EXWORKER_BINDIR}/captureworker ${EXWORKER_BINDIR}/playbackworker

development: captureworker.c
	${CC}  -O0 -g $(mycflags) $(devcflags) $(CFLAGS) $(LIBRARIES) captureworker.c mpack.c mpack_frame.c -o ${EXWORKER_BINDIR}/captureworker

${EXWORKER_BINDIR}/playbackworker: playbackworker.c mpack_frame.c
	${CC} -O0 -g $(mycflags) $(CFLAGS) $(LIBRARIES) playbackworker.c mpack.c mpack_frame.c -o ${EXWORKER_BINDIR}/playbackworker

${EXWORKER_BINDIR}/captureworker: captureworker.c mpack_frame.c exvid.c
	${CC} -O2 -g $(mycflags) $(CFLAGS) $(LIBRARIES) captureworker.c mpack.c mpack_frame.c exvid.c -o ${EXWORKER_BINDIR}/captureworker

.PHONY: clean
clean:
	-rm -f ${EXWORKER_BINDIR}/captureworker ${EXWORKER_BINDIR}/playbackworker
