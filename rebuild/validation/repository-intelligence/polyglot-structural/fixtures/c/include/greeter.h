#ifndef GREETER_H
#define GREETER_H

typedef struct Greeter {
    const char *prefix;
} Greeter;

typedef enum GreeterMode {
    GREETER_LOUD,
    GREETER_QUIET
} GreeterMode;

const char *greeter_greet(const Greeter *greeter, const char *person);

#endif
