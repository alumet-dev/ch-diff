#include <stdint.h>

typedef struct {
    uint8_t a;
    uint16_t b;
} data_t;

struct params {
    uint64_t array[10];
};

struct same {
    uint16_t *chr;
};

struct renamed {
    uint16_t *chr;
};
