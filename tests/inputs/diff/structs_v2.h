#include <cstdint>
#include <stdint.h>

typedef struct {
  uint8_t a;
  uint16_t c; // renamed
  char *str;  // added
} data_t;

struct params {
  uint64_t array[5]; // changed
};
