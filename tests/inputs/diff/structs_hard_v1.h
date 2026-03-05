#include <stdint.h>

struct complicated {
    uint8_t byte;
    struct _inline {
        uint32_t cache_size;
        uint32_t cache_properties;
        uint32_t reserved[3];
    }
    inline_struct[10];
    uint32_t reserved[15];
};

struct complicated_anon {
    uint8_t byte;
    struct {
        uint32_t cache_size;
        uint32_t cache_properties;
        uint32_t reserved[3];
    }
    inline_struct[10];
    uint32_t reserved[15];
};