package io.github.mbj.pgephemeral;

/**
 * A handle to a running ephemeral PostgreSQL server.
 *
 * <p>Implements {@link AutoCloseable} so it can be used in a try-with-resources
 * block; {@link #close()} delegates to {@link #shutdown()}.
 */
public interface Server extends AutoCloseable {
  /**
   * The PostgreSQL connection URL, e.g.
   * {@code postgres://postgres:...@127.0.0.1:54321/postgres}.
   */
  String url();

  /** Shut the server down and wait for the underlying process to exit. */
  void shutdown();

  @Override
  default void close() {
    shutdown();
  }
}
