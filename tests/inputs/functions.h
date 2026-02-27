#include <stdint.h>
#include <stdlib.h>
#include <stdbool.h>

bool no_arg(void);
int max(int a, int b);

/// Returns a pointer to an array of 3 ints
int (*complex_type(const void *p))[3];

void many_params(char* message, size_t message_len, uint64_t* out, const int array[10]);
