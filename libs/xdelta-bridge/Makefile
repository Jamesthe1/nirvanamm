LDFLAGS := -L../..
LIBS := -lxdelta3 -static-libstdc++
CC := gcc

all: bridge.c
	$(CC) $^ -shared -Wl,--subsystem,windows -o xdelta3_bridge.dll $(LDFLAGS) $(LIBS)
.PHONY: all