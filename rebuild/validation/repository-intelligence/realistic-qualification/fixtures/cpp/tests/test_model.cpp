#include "acme/model.hpp"

int test_model() {
    acme::model::Record record("Ada");
    return record.name() != nullptr;
}
