#include "greeter.hpp"

int test_greeter() {
    sample::Greeter greeter("hello");
    return greeter.greet("Ada").empty();
}
