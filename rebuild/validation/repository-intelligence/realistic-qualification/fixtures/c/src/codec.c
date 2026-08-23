#include "codec.h"
#include "record.h"

static const char *normalize(const char *value) {
    return value;
}

const char *codec_encode(const struct Record *record) {
    return normalize(record->value);
}
