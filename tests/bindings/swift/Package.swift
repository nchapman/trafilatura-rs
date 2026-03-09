// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "TrafilaturaTests",
    platforms: [.macOS(.v14)],
    targets: [
        .systemLibrary(
            name: "TrafilaturaFFI",
            path: "Sources/TrafilaturaFFI"
        ),
        .target(
            name: "Trafilatura",
            dependencies: ["TrafilaturaFFI"],
            path: "Sources/Trafilatura"
        ),
        .testTarget(
            name: "TrafilaturaTests",
            dependencies: ["Trafilatura"],
            path: "Tests/TrafilaturaTests"
        ),
    ]
)
