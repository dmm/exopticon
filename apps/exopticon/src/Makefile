mycflags= -W -Wall -Wextra -Wcast-align -Wpointer-arith -Wsign-compare -Wformat=2 \
          -Wno-format-y2k  -Wmissing-braces -Wparentheses -Wtrigraphs -Wstrict-aliasing=2 \
          -std=c99
devcflags= -fsanitize=undefined -fsanitize=address

LIBRARIES= -lavformat -lavutil -lavcodec
CC ?= gcc

all: production

production: captureserver playbackserver

development: captureserver.c
	${CC}  -O0 -g $(mycflags) $(devcflags) $(CFLAGS) $(LIBRARIES) captureserver.c bson_frame.c -o captureserver

playbackserver: playbackserver.c
	${CC} -O0 -g $(mycflags) $(CFLAGS) $(LIBRARIES) playbackserver.c -o playbackserver

captureserver: captureserver.c
	${CC} -O2 -g $(mycflags) $(CFLAGS) $(LIBRARIES) captureserver.c bson_frame.c -o captureserver

.PHONY: clean
clean:
	-rm captureserver
