# Trafilatura for Android

Extract readable content, comments, and metadata from web pages.

A high-performance Rust-based implementation with native bindings for Android. See [trafilatura-rs](https://github.com/nchapman/trafilatura-rs) on GitHub.

## Installation

```kotlin
// build.gradle.kts
dependencies {
    implementation("io.github.nchapman:trafilatura:<VERSION>")
}
```

Replace `<VERSION>` with the latest release version.

## Usage

```kotlin
import trafilatura.*

// Simple extraction
val result = extractSimple(html)
println(result.contentText)
println(result.metadata.title)

// With options
val opts = defaultOptions().copy(
    includeLinks = true,
    focus = ExtractionFocus.FAVOR_RECALL
)
val result = extract(html, opts)
```

## Supported ABIs

| ABI | Use |
|-----|-----|
| arm64-v8a | Modern phones/tablets |
| armeabi-v7a | Older 32-bit devices |
| x86_64 | Emulators |

Minimum SDK: 21 (Android 5.0)

## License

Apache-2.0
