plugins {
    kotlin("jvm") version "2.0.21"
}

kotlin {
    jvmToolchain(17)
}

repositories {
    mavenCentral()
}

dependencies {
    implementation("net.java.dev.jna:jna:5.14.0")
    testImplementation(kotlin("test"))
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.test {
    useJUnitPlatform()
    systemProperty(
        "jna.library.path",
        File(rootProject.projectDir, "../../../uniffi/target/release").absolutePath
    )
    testLogging {
        events("passed", "skipped", "failed")
        showExceptions = true
        showStackTraces = true
        exceptionFormat = org.gradle.api.tasks.testing.logging.TestExceptionFormat.FULL
    }
}
