// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "TrafilaturaTests",
    platforms: [.macOS(.v14)],
    targets: [
        .systemLibrary(
            name: "trafilatura_uniffiFFI",
            path: "Sources/trafilatura_uniffiFFI"
        ),
        .target(
            name: "Trafilatura",
            dependencies: ["trafilatura_uniffiFFI"],
            path: "Sources/Trafilatura"
        ),
        .testTarget(
            name: "TrafilaturaTests",
            dependencies: ["Trafilatura"],
            path: "Tests/TrafilaturaTests"
        ),
    ]
)
