#include "record.h"

static const char *normalize(const char *value) {
    return value;
}

const char *record_normalize(const struct Record *record) {
    return normalize(record->value);
}
