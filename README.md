# ch-diff & ch-hist

Detect breaking changes in C libraries, using only their header files (no inspection of ELF binaries).

These tools help you identify changes that could break existing code **before** they cause issues in production.
They also help you support multiple versions of a library in the case of breaking changes.

| Tool        | Purpose                                                          |
| ----------- | ---------------------------------------------------------------- |
| **ch-diff** | Compare two header files and detect breaking changes.            |
| **ch-hist** | Analyze the history of a header file to track changes over time. |

## Usage

### **ch-diff: Compare Two Headers**

```bash
ch-diff old_header.h new_header.h
```

**Output**: A detailed report of breaking changes between the two files.

### **ch-hist: Analyze the History of a Header**

```bash
ch-hist -i header-versions-dir -o output-dir --whitelist whitelist.txt
```

**Output**: A timeline of changes, highlighting breaking modifications.
