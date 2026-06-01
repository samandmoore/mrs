package io.github.mbj.pgephemeral;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

import java.nio.file.Path;
import java.sql.ResultSet;
import java.sql.Statement;
import org.junit.jupiter.api.Test;

class PgEphemeralTest {
  private static final String EXPECTED_VERSION = System.getenv("EXPECTED_PG_EPHEMERAL_VERSION");

  @Test
  void versionReturnsExpectedVersion() {
    assumeTrue(EXPECTED_VERSION != null, "EXPECTED_PG_EPHEMERAL_VERSION not set");
    assertEquals(EXPECTED_VERSION, PgEphemeral.version());
  }

  @Test
  void versionHasSemanticFormat() {
    assertTrue(PgEphemeral.version().matches("\\d+\\.\\d+\\.\\d+(-.+)?"));
  }

  @Test
  void binaryPathPointsAtAnExecutable() {
    Path binary = Path.of(PgEphemeral.binaryPath());
    assertTrue(binary.toFile().canExecute(), "binary should be executable: " + binary);
  }

  @Test
  void platformSupportedReturnsABoolean() {
    // Just asserting it runs without throwing; value depends on the host.
    PgEphemeral.platformSupported();
  }

  @Test
  void startReturnsServerWithUrl() throws Exception {
    assumeTrue(PgEphemeral.platformSupported(), "platform not supported");
    Server server = PgEphemeral.start();
    try {
      assertTrue(server.url().startsWith("postgres://"), "url was: " + server.url());
    } finally {
      server.shutdown();
    }
  }

  @Test
  void startAcceptsCustomInstanceName() throws Exception {
    assumeTrue(PgEphemeral.platformSupported(), "platform not supported");
    Server server = PgEphemeral.start(StartOptions.defaults().instanceName("custom"));
    try {
      assertNotNull(server.url());
    } finally {
      server.shutdown();
    }
  }

  @Test
  void startAcceptsCustomConfigFile() throws Exception {
    assumeTrue(PgEphemeral.platformSupported(), "platform not supported");
    String config = Path.of(System.getProperty("user.dir"), "database.toml").toString();
    Server server = PgEphemeral.start(StartOptions.defaults().config(config));
    try {
      assertTrue(server.url().startsWith("postgres://"));
    } finally {
      server.shutdown();
    }
  }

  @Test
  void withServerYieldsServerAndReturnsResult() throws Exception {
    assumeTrue(PgEphemeral.platformSupported(), "platform not supported");
    int result =
        PgEphemeral.withServer(
            server -> {
              assertTrue(server.url().startsWith("postgres://"));
              return 42;
            });
    assertEquals(42, result);
  }

  @Test
  void withConnectionYieldsConnectedClient() throws Exception {
    assumeTrue(PgEphemeral.platformSupported(), "platform not supported");
    int value =
        PgEphemeral.withConnection(
            connection -> {
              try (Statement statement = connection.createStatement();
                  ResultSet resultSet = statement.executeQuery("SELECT 1 AS value")) {
                resultSet.next();
                return resultSet.getInt("value");
              }
            });
    assertEquals(1, value);
  }

  @Test
  void withConnectionReturnsCallbackResult() throws Exception {
    assumeTrue(PgEphemeral.platformSupported(), "platform not supported");
    int result = PgEphemeral.withConnection(connection -> 42);
    assertEquals(42, result);
  }
}
