#include "greeter.h"

int test_greeter(void) {
    Greeter greeter = {"hello"};
    return greeter_greet(&greeter, "Ada") == greeter.prefix ? 0 : 1;
}
