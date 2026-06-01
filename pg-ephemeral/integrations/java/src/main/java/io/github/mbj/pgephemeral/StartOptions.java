package io.github.mbj.pgephemeral;

import java.util.Optional;

/**
 * Options for starting an ephemeral PostgreSQL server.
 *
 * <p>Instances are immutable; use the {@code with*} methods to derive a
 * modified copy. The defaults mirror the Ruby and npm integrations:
 * instance name {@code "main"} and no explicit config file.
 */
public final class StartOptions {
  private static final StartOptions DEFAULTS = new StartOptions("main", null);

  private final String instanceName;
  private final String config;

  private StartOptions(String instanceName, String config) {
    this.instanceName = instanceName;
    this.config = config;
  }

  /** Default options: instance {@code "main"}, no config file. */
  public static StartOptions defaults() {
    return DEFAULTS;
  }

  /** Target instance name from {@code database.toml}. Defaults to {@code "main"}. */
  public String instanceName() {
    return instanceName;
  }

  /** Path to a {@code database.toml} config file, if any. */
  public Optional<String> config() {
    return Optional.ofNullable(config);
  }

  /** Return a copy with the given instance name. */
  public StartOptions instanceName(String instanceName) {
    return new StartOptions(instanceName, config);
  }

  /** Return a copy with the given config file path ({@code null} to clear). */
  public StartOptions config(String config) {
    return new StartOptions(instanceName, config);
  }
}
