# C header diff

Detect breaking changes in C libraries, using only their header files (no inspection of ELF binaries).

## Detected changes

- exported symbols: global variables and functions
- changes in struct fields
- changes in function parameters
- changes in enum declarations
