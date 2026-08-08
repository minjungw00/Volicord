#include "greeter.h"

const char *greeter_greet(const Greeter *greeter, const char *person) {
    (void)person;
    return greeter->prefix;
}
