# Cognitive Substrate L0 Microkernel & L1 Kernel Services
# Stream 1: CT Lifecycle, Scheduling, Capability Engine, IPC
#
# Reference: docs/adrs/ADR-001-monorepo-organization.md
#
# Layer: L0 (Microkernel — Rust, no_std)
# Owner: Engineer 01

package(default_visibility = [
    "//services:__subpackages__",    # L1 services can see L0
    "//runtime:__subpackages__",     # L2 runtime can see L0
    "//sdk:__subpackages__",         # L3 SDK can see L0
])

# ==============================================================================
# L0 Microkernel Crates (Rust, no_std)
# ==============================================================================
# These crates form the core kernel and MUST have zero external dependencies.

rust_library(
    name = "ct_lifecycle",
    srcs = glob(["ct_lifecycle/src/**/*.rs"]),
    edition = "2024",
    crate_root = "ct_lifecycle/src/lib.rs",
    visibility = [
        "//services:__subpackages__",
        "//runtime:__subpackages__",
        "//sdk:__subpackages__",
    ],
    deps = [
        # L0 cannot depend on anything outside kernel/
        ":capability_engine",
        ":ipc_signals_exceptions",
        # domain_types is the ONLY cross-layer dependency allowed
        # (see docs/dependency_policy.md)
    ],
)

rust_library(
    name = "capability_engine",
    srcs = glob(["capability_engine/src/**/*.rs"]),
    edition = "2024",
    crate_root = "capability_engine/src/lib.rs",
    visibility = [
        "//services:__subpackages__",
        "//runtime:__subpackages__",
        "//sdk:__subpackages__",
    ],
    deps = [
        ":ct_lifecycle",  # Peer dependency (same layer)
    ],
)

rust_library(
    name = "ipc_signals_exceptions",
    srcs = glob(["ipc_signals_exceptions/src/**/*.rs"]),
    edition = "2024",
    crate_root = "ipc_signals_exceptions/src/lib.rs",
    visibility = [
        "//services:__subpackages__",
        "//runtime:__subpackages__",
        "//sdk:__subpackages__",
    ],
    deps = [
        ":ct_lifecycle",  # Peer dependency (same layer)
    ],
)

# ==============================================================================
# Tests
# ==============================================================================

rust_test(
    name = "ct_lifecycle_tests",
    srcs = glob(["ct_lifecycle/src/**/*.rs"]),
    crate = ":ct_lifecycle",
    deps = [
        ":capability_engine",
        ":ipc_signals_exceptions",
    ],
)

rust_test(
    name = "capability_engine_tests",
    srcs = glob(["capability_engine/src/**/*.rs"]),
    crate = ":capability_engine",
    deps = [
        ":ct_lifecycle",
    ],
)

rust_test(
    name = "ipc_signals_exceptions_tests",
    srcs = glob(["ipc_signals_exceptions/src/**/*.rs"]),
    crate = ":ipc_signals_exceptions",
    deps = [
        ":ct_lifecycle",
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
# - L0 exports: can be imported by L1, L2, L3
# - L0 imports: cannot import from L1, L2, or L3 (only same-layer peers)
#
# See docs/dependency_policy.md for full policy.
