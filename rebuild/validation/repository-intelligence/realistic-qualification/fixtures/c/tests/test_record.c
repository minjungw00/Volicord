#include "record.h"

int test_record(void) {
    struct Record record = {"Ada"};
    return record_normalize(&record) != 0;
}
