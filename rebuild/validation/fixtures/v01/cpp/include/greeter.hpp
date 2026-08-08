#pragma once

#include <string>

namespace sample {

class Named {
public:
    virtual ~Named() = default;
    virtual std::string name() const = 0;
};

struct Options {
    bool loud;
};

enum class Mode { Loud, Quiet };

using Identifier = std::string;

class Greeter : public Named {
public:
    explicit Greeter(std::string prefix);
    std::string name() const override;
    std::string greet(const Identifier &person) const;

private:
    std::string prefix_;
};

}  // namespace sample
