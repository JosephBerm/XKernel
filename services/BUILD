# Cognitive Substrate L1 Kernel Services
# Stream 2: GPU Acceleration, Semantic Memory, Tool Registry & Telemetry
#
# Reference: docs/adrs/ADR-001-monorepo-organization.md
#
# Layer: L1 (Kernel Services — Rust, std)
# Owner: Engineer 02

package(default_visibility = [
    "//runtime:__subpackages__",     # L2 runtime can see L1
    "//sdk:__subpackages__",         # L3 SDK can see L1
])

# ==============================================================================
# L1 Kernel Services (Rust, std)
# ==============================================================================
# These crates provide kernel services and may depend on L0 (kernel/).

rust_library(
    name = "gpu_accelerator",
    srcs = glob(["gpu_accelerator/src/**/*.rs"]),
    edition = "2024",
    crate_root = "gpu_accelerator/src/lib.rs",
    visibility = [
        "//runtime:__subpackages__",
        "//sdk:__subpackages__",
    ],
    deps = [
        # L1 dependencies
        "//kernel:capability_engine",    # L0 dependency
        "//kernel:ct_lifecycle",         # L0 dependency
        "//kernel:ipc_signals_exceptions",  # L0 dependency
        ":semantic_memory",              # L1 peer
        ":tool_registry_telemetry",      # L1 peer
        # External dependencies (allowed per dependency_policy.md)
    ],
)

rust_library(
    name = "semantic_memory",
    srcs = glob(["semantic_memory/src/**/*.rs"]),
    edition = "2024",
    crate_root = "semantic_memory/src/lib.rs",
    visibility = [
        "//runtime:__subpackages__",
        "//sdk:__subpackages__",
    ],
    deps = [
        # L1 dependencies
        "//kernel:capability_engine",    # L0 dependency
        "//kernel:ct_lifecycle",         # L0 dependency
        "//kernel:ipc_signals_exceptions",  # L0 dependency
        ":gpu_accelerator",              # L1 peer
        ":tool_registry_telemetry",      # L1 peer
    ],
)

rust_library(
    name = "tool_registry_telemetry",
    srcs = glob(["tool_registry_telemetry/src/**/*.rs"]),
    edition = "2024",
    crate_root = "tool_registry_telemetry/src/lib.rs",
    visibility = [
        "//runtime:__subpackages__",
        "//sdk:__subpackages__",
    ],
    deps = [
        # L1 dependencies
        "//kernel:capability_engine",    # L0 dependency
        "//kernel:ct_lifecycle",         # L0 dependency
        "//kernel:ipc_signals_exceptions",  # L0 dependency
        ":gpu_accelerator",              # L1 peer
        ":semantic_memory",              # L1 peer
    ],
)

# ==============================================================================
# Tests
# ==============================================================================

rust_test(
    name = "gpu_accelerator_tests",
    srcs = glob(["gpu_accelerator/src/**/*.rs"]),
    crate = ":gpu_accelerator",
    deps = [
        "//kernel:capability_engine",
        "//kernel:ct_lifecycle",
        "//kernel:ipc_signals_exceptions",
        ":semantic_memory",
        ":tool_registry_telemetry",
    ],
)

rust_test(
    name = "semantic_memory_tests",
    srcs = glob(["semantic_memory/src/**/*.rs"]),
    crate = ":semantic_memory",
    deps = [
        "//kernel:capability_engine",
        "//kernel:ct_lifecycle",
        "//kernel:ipc_signals_exceptions",
        ":gpu_accelerator",
        ":tool_registry_telemetry",
    ],
)

rust_test(
    name = "tool_registry_telemetry_tests",
    srcs = glob(["tool_registry_telemetry/src/**/*.rs"]),
    crate = ":tool_registry_telemetry",
    deps = [
        "//kernel:capability_engine",
        "//kernel:ct_lifecycle",
        "//kernel:ipc_signals_exceptions",
        ":gpu_accelerator",
        ":semantic_memory",
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
# - L1 exports: can be imported by L2, L3
# - L1 imports: can only import from L0 (kernel/) and peer L1 services
#
# See docs/dependency_policy.md for full policy.
