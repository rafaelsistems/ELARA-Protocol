# elara-ffi

Foreign Function Interface (FFI) for the ELARA Protocol - providing C bindings and platform-specific integrations for mobile SDKs and cross-language interoperability.

## Features

- **C API Bindings**: Complete C-compatible API surface
- **Mobile SDK Support**: iOS and Android integration
- **Cross-Platform**: Windows, macOS, Linux support
- **Memory Safe**: Automatic memory management
- **Thread Safe**: Concurrent access support
- **Zero-Copy**: Efficient data transfer

## Quick Start

### C Integration
```c
#include <elara.h>

// Generate identity
ElaraIdentity* identity = elara_identity_generate();

// Create session
ElaraSession* session = elara_session_create(identity, 1);

// Send message
const char* message = "Hello from C!";
elara_session_send(session, target_node, message, strlen(message));

// Cleanup
elara_session_free(session);
elara_identity_free(identity);
```

### Rust Usage
```rust
use elara_ffi::{elara_identity_generate, elara_session_create};

unsafe {
    let identity = elara_identity_generate();
    let session = elara_session_create(identity, 1);
    // Use session...
}
```

## API Reference

### Identity Management
```c
// Generate new identity
ElaraIdentity* elara_identity_generate(void);

// Free identity
void elara_identity_free(ElaraIdentity* identity);

// Get node ID from identity
ElaraNodeId elara_identity_node_id(const ElaraIdentity* identity);
```

### Session Management
```c
// Create new session
ElaraSession* elara_session_create(ElaraIdentity* identity, int port);

// Free session
void elara_session_free(ElaraSession* session);

// Process events
int elara_session_tick(ElaraSession* session);
```

### Messaging
```c
// Send message
int elara_session_send(ElaraSession* session, ElaraNodeId target, 
                       const uint8_t* data, size_t len);

// Set message callback
int elara_session_set_message_callback(ElaraSession* session,
                                      ElaraMessageCallback callback,
                                      void* user_data);

// Receive message
int elara_session_receive(ElaraSession* session, const uint8_t* data, 
                          size_t len);
```

## Platform Support

### Android
```java
// Load native library
System.loadLibrary("elara");

// JNI interface
public native long elaraIdentityGenerate();
public native long elaraSessionCreate(long identity, int port);
```

### iOS
```swift
// Import framework
import Elara

// Swift wrapper
let identity = ElaraIdentity.generate()
let session = ElaraSession(identity: identity, port: 1)
```

### Windows
```cpp
// Load DLL
HMODULE elara = LoadLibrary("elara.dll");

// Get function pointer
auto generate = GetProcAddress(elara, "elara_identity_generate");
```

## Memory Management

### Automatic Cleanup
- RAII-based resource management
- Automatic reference counting
- Memory leak detection

### Manual Management
```c
// Explicit cleanup required
ElaraIdentity* identity = elara_identity_generate();
// ... use identity ...
elara_identity_free(identity); // Required!
```

## Thread Safety

### Concurrent Access
- All functions are thread-safe
- Internal synchronization
- Lock-free operations where possible

### Example
```c
// Multiple threads can safely use same session
pthread_t threads[4];
for (int i = 0; i < 4; i++) {
    pthread_create(&threads[i], NULL, worker_thread, session);
}
```

## Error Handling

### Return Codes
```c
#define ELARA_SUCCESS 0
#define ELARA_ERROR_INVALID_PARAM -1
#define ELARA_ERROR_OUT_OF_MEMORY -2
#define ELARA_ERROR_NETWORK -3
```

### Error Strings
```c
const char* elara_error_string(int error_code);
```

## Performance

### Zero-Copy Operations
- Direct memory access where possible
- Minimal allocations
- Efficient buffer management

### Benchmarks
- **Identity Generation**: < 1ms
- **Session Creation**: < 5ms
- **Message Send**: < 100μs
- **Memory Overhead**: < 1MB per session

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.