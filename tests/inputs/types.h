#include <stdint.h>

extern int EXTERN_VAR;

uintptr_t IMPLICIT;
uint64_t EXPLICIT_ONE = 1;

extern uint32_t bits32;
extern int16_t bits16;
extern int8_t bits8;

struct t1 {
    uint8_t x, y;
};

union t2 {
    struct {uint8_t u; uint16_t v; } a;
    int8_t b;
};

typedef struct {
    uint8_t x;
    struct t1 field_struct;
    union t2 field_union;
} td1;

typedef uint8_t my_uint8_t;
typedef struct t1 my_t1_struct_t;

enum possible_values {
    A,
    B
};
