# Cognitive Substrate L2 Runtime
# Stream 3: Framework Adapters, Semantic FS, Agent Lifecycle
#
# Reference: docs/adrs/ADR-001-monorepo-organization.md
#
# Layer: L2 (Runtime — Rust & TypeScript)
# Owner: Engineer 03

package(default_visibility = [
    "//sdk:__subpackages__",  # L3 SDK can see L2
])

# ==============================================================================
# L2 Runtime Services (Rust)
# ==============================================================================
# These crates provide runtime services and may depend on L0, L1.

rust_library(
    name = "framework_adapters",
    srcs = glob(["framework_adapters/src/**/*.rs"]),
    edition = "2024",
    crate_root = "framework_adapters/src/lib.rs",
    visibility = [
        "//sdk:__subpackages__",
    ],
    deps = [
        # L2 dependencies (can depend on L0 + L1)
        "//kernel:capability_engine",    # L0
        "//kernel:ct_lifecycle",         # L0
        "//kernel:ipc_signals_exceptions",  # L0
        "//services:gpu_accelerator",    # L1
        "//services:semantic_memory",    # L1
        "//services:tool_registry_telemetry",  # L1
        ":semantic_fs_agent_lifecycle",  # L2 peer
    ],
)

rust_library(
    name = "semantic_fs_agent_lifecycle",
    srcs = glob(["semantic_fs_agent_lifecycle/src/**/*.rs"]),
    edition = "2024",
    crate_root = "semantic_fs_agent_lifecycle/src/lib.rs",
    visibility = [
        "//sdk:__subpackages__",
    ],
    deps = [
        # L2 dependencies (can depend on L0 + L1)
        "//kernel:capability_engine",    # L0
        "//kernel:ct_lifecycle",         # L0
        "//kernel:ipc_signals_exceptions",  # L0
        "//services:gpu_accelerator",    # L1
        "//services:semantic_memory",    # L1
        "//services:tool_registry_telemetry",  # L1
        ":framework_adapters",           # L2 peer
    ],
)

# ==============================================================================
# L2 TypeScript Components
# ==============================================================================
# Runtime may include TypeScript/Node.js components using rules_nodejs.
# Placeholder for future TypeScript integration.

# ts_library(
#     name = "event_service",
#     srcs = glob(["ts/event_service/**/*.ts"]),
#     deps = [
#         # TypeScript dependencies
#     ],
# )

# ==============================================================================
# Tests
# ==============================================================================

rust_test(
    name = "framework_adapters_tests",
    srcs = glob(["framework_adapters/src/**/*.rs"]),
    crate = ":framework_adapters",
    deps = [
        "//kernel:capability_engine",
        "//kernel:ct_lifecycle",
        "//kernel:ipc_signals_exceptions",
        "//services:gpu_accelerator",
        "//services:semantic_memory",
        "//services:tool_registry_telemetry",
        ":semantic_fs_agent_lifecycle",
    ],
)

rust_test(
    name = "semantic_fs_agent_lifecycle_tests",
    srcs = glob(["semantic_fs_agent_lifecycle/src/**/*.rs"]),
    crate = ":semantic_fs_agent_lifecycle",
    deps = [
        "//kernel:capability_engine",
        "//kernel:ct_lifecycle",
        "//kernel:ipc_signals_exceptions",
        "//services:gpu_accelerator",
        "//services:semantic_memory",
        "//services:tool_registry_telemetry",
        ":framework_adapters",
    ],
)

# ==============================================================================
# Documentation
# ==============================================================================

filegroup(
    name = "docs",
    srcs = glob([
        "*/README.md",
        "*/src/**/*.rs",  # Includes doc comments
    ]),
)

# ==============================================================================
# Workspace Configuration
# ==============================================================================
# This directory contains a Cargo workspace (see Cargo.toml).
# Bazel rules above mirror the Cargo workspace structure for hermetic builds.
#
# Visibility Rules:
# - L2 exports: can be imported by L3
# - L2 imports: can import from L0, L1, and peer L2 components
#
# Cross-Language Note:
# L2 may include both Rust (via rules_rust) and TypeScript (via rules_nodejs).
# Each has its own build rules but shares the same layer constraints.
#
# See docs/dependency_policy.md for full policy.
