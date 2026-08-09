#include "greeter.hpp"

#include <utility>

namespace sample {

Greeter::Greeter(std::string prefix) : prefix_(std::move(prefix)) {}

std::string Greeter::name() const {
    return prefix_;
}

std::string Greeter::greet(const Identifier &person) const {
    return name() + ", " + person;
}

}  // namespace sample
