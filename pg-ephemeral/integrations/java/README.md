# pg-ephemeral Java Package

Java/Gradle wrapper for [pg-ephemeral](https://github.com/mbj/mrs/tree/main/pg-ephemeral).
Bundles the platform-specific binary in a per-platform classifier JAR and
provides a small API for ephemeral PostgreSQL instances.

Published on Maven Central: `io.github.mbj:pg-ephemeral`.

## Installation

The native binary ships in per-platform classifier JARs
(`linux-x86_64`, `linux-aarch_64`, `osx-aarch_64`). Use the
[osdetector](https://github.com/google/osdetector-gradle-plugin) Gradle plugin
to select the right one automatically — the same convention `protoc` and gRPC
use.

```kotlin
// build.gradle.kts
plugins {
    id("com.google.osdetector") version "1.7.3"
}

dependencies {
    implementation("io.github.mbj:pg-ephemeral:0.4.0")
    runtimeOnly("io.github.mbj:pg-ephemeral:0.4.0:${osdetector.classifier}")
}
```

If you prefer not to use the plugin, add the classifier dependency for your
platform explicitly, e.g. `io.github.mbj:pg-ephemeral:0.4.0:linux-x86_64`.

## Usage

### Direct connection

`withConnection` yields a connected JDBC `Connection` and closes it (and the
server) after the callback:

```java
import io.github.mbj.pgephemeral.PgEphemeral;

int value = PgEphemeral.withConnection(connection -> {
    try (var statement = connection.createStatement();
         var rs = statement.executeQuery("SELECT 1 AS value")) {
        rs.next();
        return rs.getInt("value");
    }
});
```

### Server handle

`withServer` yields a `Server` with a `.url()`. The container shuts down after
the callback:

```java
PgEphemeral.withServer(server -> {
    System.out.println(server.url()); // postgres://postgres:...@127.0.0.1:54321/postgres
    return null;
});
```

### Options

`StartOptions` mirrors the Ruby and npm integrations:

| Option         | Description                              | Default  |
|----------------|------------------------------------------|----------|
| `instanceName` | Target instance from `database.toml`     | `"main"` |
| `config`       | Path to a `database.toml` config file    | none     |

```java
StartOptions options = StartOptions.defaults()
    .instanceName("analytics")
    .config("path/to/database.toml");

PgEphemeral.withConnection(connection -> {
    connection.createStatement().execute("SELECT 1");
    return null;
}, options);
```

### Manual lifecycle

`Server` is `AutoCloseable`, so try-with-resources works too:

```java
try (Server server = PgEphemeral.start()) {
    // ... use server.url() ...
}
```

### Utilities

```java
PgEphemeral.version();           // => "0.4.0"
PgEphemeral.platformSupported(); // => true
PgEphemeral.binaryPath();        // => "/tmp/.../pg-ephemeral"
```

## How the binary is loaded

Java cannot execute a file inside a JAR, so at runtime the matching binary is
copied out of the classifier JAR (`native/<classifier>/pg-ephemeral`) to a
temporary file, marked executable, and launched via `ProcessBuilder`. Override
the extraction directory with `-Dpg.ephemeral.tmpdir=...` if `/tmp` is mounted
`noexec`.

## Requirements

- Java >= 17
- Docker Engine 20.10+ / Docker Desktop 4.34+, or Podman 5.3+
