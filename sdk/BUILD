# Cognitive Substrate L3 SDK & Developer Tools
# Stream 4: CSCI Syscall Interface, Multi-Language SDKs, Tools
#
# Reference: docs/adrs/ADR-001-monorepo-organization.md
#
# Layer: L3 (SDK & Tools — Rust, TypeScript, C#)
# Owner: Engineer 10

package(default_visibility = ["//:__pkg__"])  # L3 is top layer

# ==============================================================================
# L3 SDK Core (Rust)
# ==============================================================================
# CSCI syscall interface and core SDK, can depend on anything.

rust_library(
    name = "csci",
    srcs = glob(["csci/src/**/*.rs"]),
    edition = "2024",
    crate_root = "csci/src/lib.rs",
    visibility = [
        "//sdk:__subpackages__",
        "//:__pkg__",
    ],
    deps = [
        # L3 can depend on L0, L1, L2, L3
        "//kernel:capability_engine",          # L0
        "//kernel:ct_lifecycle",               # L0
        "//kernel:ipc_signals_exceptions",     # L0
        "//services:gpu_accelerator",          # L1
        "//services:semantic_memory",          # L1
        "//services:tool_registry_telemetry",  # L1
        "//runtime:framework_adapters",        # L2
        "//runtime:semantic_fs_agent_lifecycle",  # L2
        ":libcognitive",                       # L3 peer
    ],
)

rust_library(
    name = "libcognitive",
    srcs = glob(["libcognitive/src/**/*.rs"]),
    edition = "2024",
    crate_root = "libcognitive/src/lib.rs",
    visibility = [
        "//sdk:__subpackages__",
        "//:__pkg__",
    ],
    deps = [
        # L3 core SDK
        ":csci",  # L3 peer
    ],
)

# ==============================================================================
# Developer Tools
# ==============================================================================
# Command-line tools for tracing, profiling, and debugging.

rust_binary(
    name = "cs_trace",
    srcs = glob(["tools/cs-trace/src/**/*.rs"]),
    edition = "2024",
    crate_root = "tools/cs-trace/src/main.rs",
    deps = [
        ":csci",
        ":libcognitive",
        # Tool dependencies
    ],
)

rust_binary(
    name = "cs_replay",
    srcs = glob(["tools/cs-replay/src/**/*.rs"]),
    edition = "2024",
    crate_root = "tools/cs-replay/src/main.rs",
    deps = [
        ":csci",
        ":libcognitive",
    ],
)

rust_binary(
    name = "cs_profile",
    srcs = glob(["tools/cs-profile/src/**/*.rs"]),
    edition = "2024",
    crate_root = "tools/cs-profile/src/main.rs",
    deps = [
        ":csci",
        ":libcognitive",
    ],
)

rust_binary(
    name = "cs_capgraph",
    srcs = glob(["tools/cs-capgraph/src/**/*.rs"]),
    edition = "2024",
    crate_root = "tools/cs-capgraph/src/main.rs",
    deps = [
        ":csci",
        ":libcognitive",
    ],
)

rust_binary(
    name = "cs_top",
    srcs = glob(["tools/cs-top/src/**/*.rs"]),
    edition = "2024",
    crate_root = "tools/cs-top/src/main.rs",
    deps = [
        ":csci",
        ":libcognitive",
    ],
)

# ==============================================================================
# L3 TypeScript SDK
# ==============================================================================
# Multi-language SDK published to npm and NuGet.
# Placeholder for rules_nodejs integration.

# ts_library(
#     name = "sdk_ts",
#     srcs = glob(["ts/sdk/**/*.ts"]),
#     package_json = "ts/package.json",
#     deps = [
#         # TypeScript SDK dependencies
#     ],
# )

# ts_library(
#     name = "cli_ts",
#     srcs = glob(["ts/cli/**/*.ts"]),
#     package_json = "ts/package.json",
#     deps = [
#         ":sdk_ts",
#     ],
# )

# ==============================================================================
# L3 C# SDK
# ==============================================================================
# .NET SDK with Semantic Kernel integration.
# Placeholder for rules_dotnet integration.

# csharp_library(
#     name = "sdk_cs",
#     srcs = glob(["csharp/CognitiveSubstrate.SDK/**/*.cs"]),
#     target_framework = "net8.0",
#     deps = [
#         # C# SDK dependencies
#     ],
# )

# ==============================================================================
# Tests
# ==============================================================================

rust_test(
    name = "csci_tests",
    srcs = glob(["csci/src/**/*.rs"]),
    crate = ":csci",
    deps = [
        ":libcognitive",
        "//kernel:capability_engine",
        "//kernel:ct_lifecycle",
        "//kernel:ipc_signals_exceptions",
        "//services:gpu_accelerator",
        "//services:semantic_memory",
        "//services:tool_registry_telemetry",
        "//runtime:framework_adapters",
        "//runtime:semantic_fs_agent_lifecycle",
    ],
)

rust_test(
    name = "libcognitive_tests",
    srcs = glob(["libcognitive/src/**/*.rs"]),
    crate = ":libcognitive",
    deps = [
        ":csci",
    ],
)

# ==============================================================================
# Documentation
# ==============================================================================

filegroup(
    name = "docs",
    srcs = glob([
        "*/README.md",
        "tools/*/README.md",
        "*/src/**/*.rs",  # Includes doc comments
        "ts/**/*.md",
        "csharp/**/*.md",
    ]),
)

# ==============================================================================
# Published Artifacts
# ==============================================================================
# Targets for publishing SDKs to package registries.

# For npm (TypeScript SDK)
# npm_package(
#     name = "sdk_npm",
#     srcs = [":sdk_ts"],
#     package_name = "@cognitive-substrate/sdk",
# )

# For NuGet (C# SDK)
# nuget_package(
#     name = "sdk_nuget",
#     srcs = [":sdk_cs"],
#     package_id = "CognitiveSubstrate.SDK",
# )

# ==============================================================================
# Workspace Configuration
# ==============================================================================
# This directory contains multiple Cargo workspaces:
# - Cargo.toml (Rust SDK)
# - ts/package.json (TypeScript SDK, npm monorepo)
# - csharp/CognitiveSubstrate.SDK.sln (C# SDK)
#
# Bazel rules above provide hermetic, language-agnostic build orchestration.
#
# Visibility Rules:
# - L3 exports: can be imported by anything (top layer)
# - L3 imports: can import from L0, L1, L2, and peer L3 components
#
# See docs/dependency_policy.md for full policy.
