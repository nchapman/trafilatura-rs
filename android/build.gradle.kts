plugins {
    id("com.android.library") version "9.1.0"
    id("com.vanniktech.maven.publish") version "0.36.0"
}

android {
    namespace = "io.github.nchapman.trafilatura"
    compileSdk = 35

    defaultConfig {
        minSdk = 21
        consumerProguardFiles("proguard-rules.pro")
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }

    kotlin {
        compilerOptions {
            jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_1_8)
        }
    }
}

dependencies {
    implementation("net.java.dev.jna:jna:5.14.0@aar")
}

mavenPublishing {
    publishToMavenCentral()
    signAllPublications()

    coordinates("io.github.nchapman", "trafilatura", project.findProperty("VERSION")?.toString() ?: "0.0.0-dev")

    pom {
        name.set("Trafilatura")
        description.set("Extract readable content, comments, and metadata from web pages. High-performance Rust implementation with Android native bindings.")
        url.set("https://github.com/nchapman/trafilatura-rs")
        licenses {
            license {
                name.set("Apache-2.0")
                url.set("https://www.apache.org/licenses/LICENSE-2.0")
            }
        }
        developers {
            developer {
                id.set("nchapman")
                name.set("Nathaniel Chapman")
            }
        }
        scm {
            url.set("https://github.com/nchapman/trafilatura-rs")
            connection.set("scm:git:git://github.com/nchapman/trafilatura-rs.git")
            developerConnection.set("scm:git:ssh://git@github.com/nchapman/trafilatura-rs.git")
        }
    }
}
