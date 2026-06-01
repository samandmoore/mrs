package io.github.mbj.pgephemeral;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.OutputStream;
import java.io.UncheckedIOException;
import java.net.URI;
import java.net.URLDecoder;
import java.nio.charset.StandardCharsets;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;
import java.util.ArrayList;
import java.util.List;
import java.util.Properties;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Java wrapper around the {@code pg-ephemeral} binary, providing ephemeral
 * PostgreSQL instances for testing.
 *
 * <p>The API mirrors the Ruby and npm integrations: {@link #start},
 * {@link #withServer}, and {@link #withConnection} boot a throwaway PostgreSQL
 * container and tear it down afterwards.
 */
public final class PgEphemeral {
  /** Callback receiving a running {@link Server}. */
  @FunctionalInterface
  public interface ServerFunction<T> {
    T apply(Server server) throws Exception;
  }

  /** Callback receiving a connected JDBC {@link Connection}. */
  @FunctionalInterface
  public interface ConnectionFunction<T> {
    T apply(Connection connection) throws Exception;
  }

  private static final Pattern VERSION_PATTERN =
      Pattern.compile("^pg-ephemeral (\\d+\\.\\d+\\.\\d+(?:-.+)?)\\R?$");
  private static final Pattern URL_FIELD_PATTERN =
      Pattern.compile("\"url\"\\s*:\\s*\"((?:[^\"\\\\]|\\\\.)*)\"");

  private PgEphemeral() {}

  /** Absolute path to the extracted, executable {@code pg-ephemeral} binary. */
  public static String binaryPath() {
    return NativeLoader.binaryPath().toString();
  }

  /** The version reported by the bundled binary, e.g. {@code "0.4.0"}. */
  public static String version() {
    String output = runCapture(binaryPath(), "--version");
    Matcher matcher = VERSION_PATTERN.matcher(output.trim());
    if (!matcher.matches()) {
      throw new IllegalStateException("Failed to parse version from pg-ephemeral binary: " + output);
    }
    return matcher.group(1);
  }

  /** Whether the current platform supports running ephemeral instances. */
  public static boolean platformSupported() {
    try {
      Process process =
          new ProcessBuilder(binaryPath(), "platform", "support")
              .redirectOutput(ProcessBuilder.Redirect.DISCARD)
              .redirectError(ProcessBuilder.Redirect.DISCARD)
              .start();
      return process.waitFor() == 0;
    } catch (IOException error) {
      throw new UncheckedIOException(error);
    } catch (InterruptedException error) {
      Thread.currentThread().interrupt();
      throw new IllegalStateException("Interrupted while checking platform support", error);
    }
  }

  /** Start a server with default options. */
  public static Server start() {
    return start(StartOptions.defaults());
  }

  /** Start a server with the given options. */
  public static Server start(StartOptions options) {
    List<String> command = new ArrayList<>();
    command.add(binaryPath());
    options.config().ifPresent(config -> {
      command.add("--config-file");
      command.add(config);
    });
    command.add("integration-server");
    command.add("--instance");
    command.add(options.instanceName());
    // The binary writes the result JSON to --result-fd and shuts down on EOF of
    // --control-fd. Map these onto stdout (1) and stdin (0): logs go to stderr,
    // so stdout carries only the JSON line, and closing stdin triggers shutdown.
    command.add("--result-fd");
    command.add("1");
    command.add("--control-fd");
    command.add("0");

    Process process;
    try {
      process =
          new ProcessBuilder(command).redirectError(ProcessBuilder.Redirect.INHERIT).start();
    } catch (IOException error) {
      throw new UncheckedIOException("Failed to start pg-ephemeral", error);
    }

    String configLine;
    try {
      BufferedReader reader =
          new BufferedReader(
              new InputStreamReader(process.getInputStream(), StandardCharsets.UTF_8));
      configLine = reader.readLine();
    } catch (IOException error) {
      process.destroyForcibly();
      throw new UncheckedIOException("Failed to read server configuration", error);
    }

    if (configLine == null) {
      process.destroyForcibly();
      throw new IllegalStateException("pg-ephemeral exited before reporting server configuration");
    }

    String url = extractUrl(configLine);
    return new ProcessServer(process, url);
  }

  /** Run {@code function} with a started server, shutting it down afterwards. */
  public static <T> T withServer(ServerFunction<T> function) throws Exception {
    return withServer(function, StartOptions.defaults());
  }

  /** Run {@code function} with a started server, shutting it down afterwards. */
  public static <T> T withServer(ServerFunction<T> function, StartOptions options)
      throws Exception {
    Server server = start(options);
    try {
      return function.apply(server);
    } finally {
      server.shutdown();
    }
  }

  /** Run {@code function} with a connected JDBC connection, cleaning up afterwards. */
  public static <T> T withConnection(ConnectionFunction<T> function) throws Exception {
    return withConnection(function, StartOptions.defaults());
  }

  /** Run {@code function} with a connected JDBC connection, cleaning up afterwards. */
  public static <T> T withConnection(ConnectionFunction<T> function, StartOptions options)
      throws Exception {
    return withServer(
        server -> {
          try (Connection connection = connect(server.url())) {
            return function.apply(connection);
          }
        },
        options);
  }

  private static Connection connect(String postgresUrl) throws SQLException {
    URI uri = URI.create(postgresUrl);
    String host = uri.getHost();
    int port = uri.getPort();
    String database = uri.getPath() == null ? "" : uri.getPath().replaceFirst("^/", "");

    StringBuilder jdbcUrl = new StringBuilder("jdbc:postgresql://").append(host);
    if (port != -1) {
      jdbcUrl.append(':').append(port);
    }
    jdbcUrl.append('/').append(database);

    Properties properties = new Properties();
    String userInfo = uri.getUserInfo();
    if (userInfo != null) {
      int separator = userInfo.indexOf(':');
      if (separator >= 0) {
        properties.setProperty("user", decode(userInfo.substring(0, separator)));
        properties.setProperty("password", decode(userInfo.substring(separator + 1)));
      } else {
        properties.setProperty("user", decode(userInfo));
      }
    }

    String query = uri.getQuery();
    if (query != null && !query.isEmpty()) {
      for (String pair : query.split("&")) {
        int separator = pair.indexOf('=');
        if (separator >= 0) {
          properties.setProperty(
              decode(pair.substring(0, separator)), decode(pair.substring(separator + 1)));
        }
      }
    }

    return DriverManager.getConnection(jdbcUrl.toString(), properties);
  }

  private static String decode(String value) {
    return URLDecoder.decode(value, StandardCharsets.UTF_8);
  }

  private static String extractUrl(String configLine) {
    Matcher matcher = URL_FIELD_PATTERN.matcher(configLine);
    if (!matcher.find()) {
      throw new IllegalStateException("No 'url' field in server configuration: " + configLine);
    }
    return matcher.group(1).replace("\\\"", "\"").replace("\\\\", "\\");
  }

  private static String runCapture(String... command) {
    try {
      Process process =
          new ProcessBuilder(command).redirectError(ProcessBuilder.Redirect.INHERIT).start();
      String output;
      try (InputStream stream = process.getInputStream()) {
        output = new String(stream.readAllBytes(), StandardCharsets.UTF_8);
      }
      int status = process.waitFor();
      if (status != 0) {
        throw new IllegalStateException(
            "Command " + String.join(" ", command) + " failed with exit code " + status);
      }
      return output;
    } catch (IOException error) {
      throw new UncheckedIOException(error);
    } catch (InterruptedException error) {
      Thread.currentThread().interrupt();
      throw new IllegalStateException("Interrupted while running pg-ephemeral", error);
    }
  }

  private static final class ProcessServer implements Server {
    private final Process process;
    private final String url;

    ProcessServer(Process process, String url) {
      this.process = process;
      this.url = url;
    }

    @Override
    public String url() {
      return url;
    }

    @Override
    public void shutdown() {
      if (!process.isAlive()) {
        return;
      }
      try (OutputStream control = process.getOutputStream()) {
        control.close();
      } catch (IOException error) {
        throw new UncheckedIOException("Failed to signal pg-ephemeral shutdown", error);
      }
      try {
        process.waitFor();
      } catch (InterruptedException error) {
        Thread.currentThread().interrupt();
        process.destroyForcibly();
        throw new IllegalStateException("Interrupted while shutting down pg-ephemeral", error);
      }
    }
  }
}
