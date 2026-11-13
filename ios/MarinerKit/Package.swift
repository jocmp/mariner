// swift-tools-version: 6.2

import PackageDescription

/// Mariner's Rust core for the app: the XCFramework and its generated
/// Swift bindings, both written here by `make ios-kit`.
let package = Package(
    name: "MarinerKit",
    platforms: [.iOS(.v18)],
    products: [
        .library(name: "Mariner", targets: ["Mariner"]),
    ],
    targets: [
        .binaryTarget(name: "MarinerFFI", path: "MarinerFFI.xcframework"),
        .target(
            name: "Mariner",
            dependencies: ["MarinerFFI"],
            // UniFFI's generated async callbacks don't pass Swift 6's
            // data-race checks yet. The app itself builds in Swift 6 mode.
            swiftSettings: [.swiftLanguageMode(.v5)]
        ),
    ]
)
