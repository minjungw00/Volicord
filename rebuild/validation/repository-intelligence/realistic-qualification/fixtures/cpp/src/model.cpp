#include "acme/model.hpp"

namespace acme::model {
const char *normalize(const char *value) { return value; }

Record::Record(const char *value) : value_(value) {}

const char *Record::name() const {
    return normalize(value_);
}
}
