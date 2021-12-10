mycflags= -W -Wall -Wextra -Wcast-align -Wpointer-arith -Wsign-compare -Wformat=2 \
          -Wno-format-y2k  -Wmissing-braces -Wparentheses -Wtrigraphs -Wstrict-aliasing=2 \
          -std=c99 -flto -D_FORTIFY_SOURCE=2 -fstack-protector-strong
devcflags= -fsanitize=undefined -fsanitize=address

LIBRARIES= -lavformat -lavutil -lavcodec -lswscale -lturbojpeg
CC ?= clang

# so we can run make outside of Cargo
CARGO_PROFILE ?= debug
OUTDIR ?= ../../target/assets
CARGO_TARGET_DIR ?= ../../target
EXSERIAL_LIB= $(CARGO_TARGET_DIR)/$(CARGO_PROFILE)/libexserial.a
EXSERIAL_FLAGS= -L$(CARGO_TARGET_DIR) -ldl -lpthread $(EXSERIAL_LIB)
EXWORKER_BINDIR ?= $(OUTDIR)/workers/cworkers

all: ${EXWORKER_BINDIR}/captureworker ${EXWORKER_BINDIR}/playbackworker

development: captureworker.c exvid.c $(EXSERIAL_LIB)
	mkdir -p ${EXWORKER_BINDIR}
	${CC} -O0 -g $(mycflags) $(CFLAGS) captureworker.c exvid.c -o ${EXWORKER_BINDIR}/captureworker $(EXSERIAL_FLAGS) $(LIBRARIES)


${EXWORKER_BINDIR}/playbackworker: playbackworker.c $(EXSERIAL_LIB)
	mkdir -p ${EXWORKER_BINDIR}
	${CC} -O2 -g $(mycflags) $(CFLAGS) playbackworker.c -o ${EXWORKER_BINDIR}/playbackworker $(EXSERIAL_FLAGS) $(LIBRARIES)

${EXWORKER_BINDIR}/captureworker: captureworker.c exvid.c $(EXSERIAL_LIB)
	mkdir -p ${EXWORKER_BINDIR}
	${CC} -O2 -g $(mycflags) $(CFLAGS) captureworker.c exvid.c -o ${EXWORKER_BINDIR}/captureworker $(EXSERIAL_FLAGS) $(LIBRARIES)

.PHONY: clean
clean:
	-rm -f ${EXWORKER_BINDIR}/captureworker ${EXWORKER_BINDIR}/playbackworker
