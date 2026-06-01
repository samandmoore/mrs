// The `version` below is synced from the Rust source of truth by
// `manager pg-ephemeral java sync`. Do not edit the version manually.

plugins {
    `java-library`
    id("com.vanniktech.maven.publish") version "0.30.0"
}

group = "io.github.mbj"
version = "0.4.0"

repositories {
    mavenCentral()
}

java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
}

dependencies {
    implementation("org.postgresql:postgresql:42.7.4")

    testImplementation(platform("org.junit:junit-bom:5.10.2"))
    testImplementation("org.junit.jupiter:junit-jupiter")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

// Binaries are staged here by `manager pg-ephemeral java {build,merge}` as
// `native/<classifier>/pg-ephemeral`. The main JAR stays platform-independent;
// each classifier JAR packages exactly one platform's binary as a resource.
val nativeStaging: Provider<Directory> = layout.buildDirectory.dir("native-staging")

val classifiers = listOf("linux-x86_64", "linux-aarch_64", "osx-aarch_64")

val nativeJarTasks = classifiers.map { classifier ->
    val taskName = "nativeJar" + classifier.filter(Char::isLetterOrDigit).replaceFirstChar(Char::uppercase)
    tasks.register<Jar>(taskName) {
        archiveClassifier.set(classifier)
        val binary = nativeStaging.map { it.file("native/$classifier/pg-ephemeral") }
        from(nativeStaging.map { it.dir("native/$classifier") }) {
            into("native/$classifier")
        }
        onlyIf { binary.get().asFile.exists() }
    }
}

tasks.named("assemble") {
    dependsOn(nativeJarTasks)
}

tasks.test {
    useJUnitPlatform()
    // Tests run against the real per-platform classifier JAR (the consumer
    // path): `manager pg-ephemeral java test` passes its absolute path here so
    // the binary is resolved from `native/<classifier>/pg-ephemeral`.
    providers.gradleProperty("pgEphemeralClassifierJar").orNull?.let { jar ->
        classpath += files(jar)
    }
    providers.environmentVariable("EXPECTED_PG_EPHEMERAL_VERSION").orNull?.let {
        environment("EXPECTED_PG_EPHEMERAL_VERSION", it)
    }
    testLogging {
        events("passed", "skipped", "failed")
    }
}

mavenPublishing {
    publishToMavenCentral()
    signAllPublications()
    coordinates(group.toString(), "pg-ephemeral", version.toString())

    pom {
        name.set("pg-ephemeral")
        description.set(
            "Provides ephemeral PostgreSQL instances for testing, wrapping the pg-ephemeral project binary"
        )
        url.set("https://github.com/mbj/mrs/tree/main/pg-ephemeral")
        licenses {
            license {
                name.set("MIT")
                url.set("https://opensource.org/licenses/MIT")
            }
        }
        developers {
            developer {
                id.set("mbj")
                name.set("Markus Schirp")
                email.set("mbj@schirp-dso.com")
            }
        }
        scm {
            url.set("https://github.com/mbj/mrs")
            connection.set("scm:git:https://github.com/mbj/mrs.git")
            developerConnection.set("scm:git:ssh://git@github.com/mbj/mrs.git")
        }
    }
}

// Attach the per-platform classifier JARs to the published module so all
// platforms share one groupId:artifactId:version (the protoc/osdetector
// convention). Consumers select theirs via the osdetector Gradle plugin.
afterEvaluate {
    publishing.publications.withType<MavenPublication>().configureEach {
        nativeJarTasks.forEach { artifact(it) }
    }
}
