mycflags= -W -Wall -Wextra -Wcast-align -Wpointer-arith -Wsign-compare -Wformat=2 \
          -Wno-format-y2k  -Wmissing-braces -Wparentheses -Wtrigraphs -Wstrict-aliasing=2 \
          -std=c99 -flto -D_FORTIFY_SOURCE=2 -fstack-protector-strong
devcflags= -fsanitize=undefined -fsanitize=address

LIBRARIES= -lavformat -lavutil -lavcodec -lswscale -lturbojpeg
CC ?= clang

# so we can run make outside of Cargo
CARGO_TARGET_DIR ?= "../../../target/debug/"
EXSERIAL_FLAGS= -L$(CARGO_TARGET_DIR) -lexserial
EXWORKER_BINDIR ?= .

all: ${EXWORKER_BINDIR}/captureworker ${EXWORKER_BINDIR}/playbackworker

development: captureworker.c
	${CC}  -O0 -g $(mycflags) $(devcflags) $(CFLAGS) $(LIBRARIES) captureworker.c -o ${EXWORKER_BINDIR}/captureworker $(EXSERIAL_FLAGS)

${EXWORKER_BINDIR}/playbackworker: playbackworker.c mpack_frame.c
	${CC} -O2 -g $(mycflags) $(CFLAGS) $(LIBRARIES) playbackworker.c -o ${EXWORKER_BINDIR}/playbackworker $(EXSERIAL_FLAGS)

${EXWORKER_BINDIR}/captureworker: captureworker.c mpack_frame.c exvid.c
	${CC} -O2 -g $(mycflags) $(CFLAGS) $(LIBRARIES) captureworker.c exvid.c -o ${EXWORKER_BINDIR}/captureworker $(EXSERIAL_FLAGS)

.PHONY: clean
clean:
	-rm -f ${EXWORKER_BINDIR}/captureworker ${EXWORKER_BINDIR}/playbackworker
