// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "EdgeQuakeSDK",
    platforms: [.macOS(.v13)],
    products: [
        .library(name: "EdgeQuakeSDK", targets: ["EdgeQuakeSDK"]),
    ],
    targets: [
        .target(name: "EdgeQuakeSDK"),
        .testTarget(name: "EdgeQuakeSDKTests", dependencies: ["EdgeQuakeSDK"]),
    ]
)
