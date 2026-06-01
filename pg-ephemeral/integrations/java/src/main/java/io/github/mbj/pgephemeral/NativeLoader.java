package io.github.mbj.pgephemeral;

import java.io.IOException;
import java.io.InputStream;
import java.io.UncheckedIOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardCopyOption;
import java.util.Locale;

/**
 * Locates and extracts the platform-specific {@code pg-ephemeral} binary.
 *
 * <p>Java cannot execute a file that lives inside a JAR, so the binary is
 * shipped as a classpath resource ({@code native/<classifier>/pg-ephemeral})
 * inside a per-platform classifier JAR. At runtime the matching binary is
 * copied to a temporary file, marked executable, and the path returned for use
 * with {@link ProcessBuilder}. This mirrors the approach used by sqlite-jdbc's
 * {@code SQLiteJDBCLoader} for native libraries.
 */
final class NativeLoader {
  /** System property to override the directory used for binary extraction. */
  static final String TMPDIR_PROPERTY = "pg.ephemeral.tmpdir";

  private static volatile Path cachedBinary;

  private NativeLoader() {}

  /** Resolve the path to the extracted, executable binary (cached per JVM). */
  static Path binaryPath() {
    Path cached = cachedBinary;
    if (cached != null && Files.exists(cached)) {
      return cached;
    }
    synchronized (NativeLoader.class) {
      if (cachedBinary != null && Files.exists(cachedBinary)) {
        return cachedBinary;
      }
      cachedBinary = extract();
      return cachedBinary;
    }
  }

  /**
   * The osdetector-style platform classifier for the current host, e.g.
   * {@code linux-x86_64}, {@code linux-aarch_64}, {@code osx-aarch_64}.
   */
  static String classifier() {
    String osName = System.getProperty("os.name", "").toLowerCase(Locale.ROOT);
    String osArch = System.getProperty("os.arch", "").toLowerCase(Locale.ROOT);

    String os;
    if (osName.contains("linux")) {
      os = "linux";
    } else if (osName.contains("mac") || osName.contains("darwin") || osName.contains("osx")) {
      os = "osx";
    } else {
      throw new UnsupportedOperationException("Unsupported operating system: " + osName);
    }

    String arch;
    if (osArch.equals("x86_64") || osArch.equals("amd64")) {
      arch = "x86_64";
    } else if (osArch.equals("aarch64") || osArch.equals("arm64")) {
      arch = "aarch_64";
    } else {
      throw new UnsupportedOperationException("Unsupported CPU architecture: " + osArch);
    }

    return os + "-" + arch;
  }

  private static Path extract() {
    String classifier = classifier();
    String resource = "native/" + classifier + "/pg-ephemeral";

    ClassLoader loader = NativeLoader.class.getClassLoader();
    try (InputStream in = loader.getResourceAsStream(resource)) {
      if (in == null) {
        throw new IllegalStateException(
            "pg-ephemeral binary not found on classpath at '"
                + resource
                + "'. Add the platform dependency "
                + "io.github.mbj:pg-ephemeral:<version>:"
                + classifier
                + " (the osdetector Gradle plugin resolves it automatically).");
      }

      Path directory = createExtractionDirectory();
      Path binary = directory.resolve("pg-ephemeral");
      Files.copy(in, binary, StandardCopyOption.REPLACE_EXISTING);

      if (!binary.toFile().setExecutable(true, true)) {
        throw new IllegalStateException("Failed to mark binary executable: " + binary);
      }

      binary.toFile().deleteOnExit();
      directory.toFile().deleteOnExit();
      return binary;
    } catch (IOException error) {
      throw new UncheckedIOException("Failed to extract pg-ephemeral binary", error);
    }
  }

  private static Path createExtractionDirectory() throws IOException {
    String override = System.getProperty(TMPDIR_PROPERTY);
    if (override != null && !override.isEmpty()) {
      Path base = Paths.get(override);
      Files.createDirectories(base);
      return Files.createTempDirectory(base, "pg-ephemeral");
    }
    return Files.createTempDirectory("pg-ephemeral");
  }
}
