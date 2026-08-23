#ifndef RECORD_H
#define RECORD_H

#define RECORD_CAPACITY 64

struct Record {
    const char *value;
};

#ifdef RECORD_ENABLE_TRACE
void record_trace(const struct Record *record);
#endif

const char *record_normalize(const struct Record *record);

#endif
