#include <stdio.h>

typedef void (*xprintf_ptr)(const char* msg);

extern int xd3_main_cmdline(int argc, char** argv);
extern xprintf_ptr xprintf_message_func;

int xd3_call(int argc, char** argv, xprintf_ptr msg_collector) {
    xprintf_message_func = msg_collector;
    int ret = xd3_main_cmdline(argc, argv);
    xprintf_message_func = NULL;
    return ret;
}

// TODO: Encode and decode functions