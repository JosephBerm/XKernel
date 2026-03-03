# Cognitive Substrate L3 Developer Tools
# Stream 4, L3 Layer: Command-line utilities for tracing, profiling, debugging
#
# Week 4 Enhancement: Full BUILD file with visibility rules and tool targets
#
# Reference: docs/adrs/ADR-001-monorepo-organization.md
# Layer: L3 (SDK & Tools — Rust)
# Owner: Engineer 10 (Tooling, Packaging & Documentation)

package(default_visibility = [
    "//:__pkg__",        # Root workspace can reference tools
    "//sdk:__pkg__",     # SDK layer exports
])

# ==============================================================================
# Tool Dependencies
# ==============================================================================
# All tools depend on CSCI and libcognitive for kernel access.

_TOOL_DEPS = [
    "//sdk:csci",                    # CSCI syscall interface
    "//sdk:libcognitive",            # SDK core
]

# ==============================================================================
# cs-trace: Execution Trace Capture Tool
# ==============================================================================
# Captures and records kernel event traces for later analysis.
#
# Features:
#   - Low-overhead event capture
#   - Ring buffer for circular logging
#   - Multiple output formats (binary, CSV, JSON)
#
# See: sdk/tools/cs-trace/README.md

rust_library(
    name = "cs_trace_lib",
    srcs = glob(["cs-trace/src/**/*.rs"]),
    edition = "2024",
    crate_root = "cs-trace/src/lib.rs",
    crate_name = "cs_trace",
    visibility = [
        "//:__pkg__",
        "//sdk:__subpackages__",
    ],
    deps = _TOOL_DEPS,
)

rust_binary(
    name = "cs_trace",
    srcs = glob(["cs-trace/src/**/*.rs"]),
    edition = "2024",
    crate_root = "cs-trace/src/main.rs",
    visibility = [
        "//:__pkg__",
        "//sdk:__subpackages__",
    ],
    deps = _TOOL_DEPS + [
        ":cs_trace_lib",
    ],
)

rust_test(
    name = "cs_trace_tests",
    srcs = glob(["cs-trace/src/**/*.rs"]),
    crate = ":cs_trace_lib",
    deps = _TOOL_DEPS,
)

# ==============================================================================
# cs-replay: Trace Replay & Analysis Tool
# ==============================================================================
# Replays captured execution traces for debugging and analysis.
#
# Features:
#   - Trace playback with speed control
#   - Breakpoint support
#   - Event filtering
#   - State inspection
#
# See: sdk/tools/cs-replay/README.md

rust_library(
    name = "cs_replay_lib",
    srcs = glob(["cs-replay/src/**/*.rs"]),
    edition = "2024",
    crate_root = "cs-replay/src/lib.rs",
    crate_name = "cs_replay",
    visibility = [
        "//:__pkg__",
        "//sdk:__subpackages__",
    ],
    deps = _TOOL_DEPS,
)

rust_binary(
    name = "cs_replay",
    srcs = glob(["cs-replay/src/**/*.rs"]),
    edition = "2024",
    crate_root = "cs-replay/src/main.rs",
    visibility = [
        "//:__pkg__",
        "//sdk:__subpackages__",
    ],
    deps = _TOOL_DEPS + [
        ":cs_replay_lib",
    ],
)

rust_test(
    name = "cs_replay_tests",
    srcs = glob(["cs-replay/src/**/*.rs"]),
    crate = ":cs_replay_lib",
    deps = _TOOL_DEPS,
)

# ==============================================================================
# cs-profile: Performance Profiling Tool
# ==============================================================================
# Collects performance metrics and generates profile reports.
#
# Features:
#   - CPU time sampling
#   - Memory usage tracking
#   - Capability acquisition analysis
#   - Call graph reconstruction
#
# See: sdk/tools/cs-profile/README.md

rust_library(
    name = "cs_profile_lib",
    srcs = glob(["cs-profile/src/**/*.rs"]),
    edition = "2024",
    crate_root = "cs-profile/src/lib.rs",
    crate_name = "cs_profile",
    visibility = [
        "//:__pkg__",
        "//sdk:__subpackages__",
    ],
    deps = _TOOL_DEPS,
)

rust_binary(
    name = "cs_profile",
    srcs = glob(["cs-profile/src/**/*.rs"]),
    edition = "2024",
    crate_root = "cs-profile/src/main.rs",
    visibility = [
        "//:__pkg__",
        "//sdk:__subpackages__",
    ],
    deps = _TOOL_DEPS + [
        ":cs_profile_lib",
    ],
)

rust_test(
    name = "cs_profile_tests",
    srcs = glob(["cs-profile/src/**/*.rs"]),
    crate = ":cs_profile_lib",
    deps = _TOOL_DEPS,
)

# ==============================================================================
# cs-capgraph: Capability Graph Visualization Tool
# ==============================================================================
# Visualizes the capability grant graph for security analysis.
#
# Features:
#   - Graph generation (GraphViz, JSON, SVG)
#   - Reachability analysis
#   - Confinement verification
#   - Delegation tracking
#
# See: sdk/tools/cs-capgraph/README.md

rust_library(
    name = "cs_capgraph_lib",
    srcs = glob(["cs-capgraph/src/**/*.rs"]),
    edition = "2024",
    crate_root = "cs-capgraph/src/lib.rs",
    crate_name = "cs_capgraph",
    visibility = [
        "//:__pkg__",
        "//sdk:__subpackages__",
    ],
    deps = _TOOL_DEPS,
)

rust_binary(
    name = "cs_capgraph",
    srcs = glob(["cs-capgraph/src/**/*.rs"]),
    edition = "2024",
    crate_root = "cs-capgraph/src/main.rs",
    visibility = [
        "//:__pkg__",
        "//sdk:__subpackages__",
    ],
    deps = _TOOL_DEPS + [
        ":cs_capgraph_lib",
    ],
)

rust_test(
    name = "cs_capgraph_tests",
    srcs = glob(["cs-capgraph/src/**/*.rs"]),
    crate = ":cs_capgraph_lib",
    deps = _TOOL_DEPS,
)

# ==============================================================================
# cs-top: System Monitor Tool
# ==============================================================================
# Real-time system monitor similar to 'top', but for CS kernel metrics.
#
# Features:
#   - Real-time capability usage
#   - CT lifecycle state monitoring
#   - IPC activity tracking
#   - GPU accelerator metrics
#   - Memory statistics
#
# See: sdk/tools/cs-top/README.md

rust_library(
    name = "cs_top_lib",
    srcs = glob(["cs-top/src/**/*.rs"]),
    edition = "2024",
    crate_root = "cs-top/src/lib.rs",
    crate_name = "cs_top",
    visibility = [
        "//:__pkg__",
        "//sdk:__subpackages__",
    ],
    deps = _TOOL_DEPS,
)

rust_binary(
    name = "cs_top",
    srcs = glob(["cs-top/src/**/*.rs"]),
    edition = "2024",
    crate_root = "cs-top/src/main.rs",
    visibility = [
        "//:__pkg__",
        "//sdk:__subpackages__",
    ],
    deps = _TOOL_DEPS + [
        ":cs_top_lib",
    ],
)

rust_test(
    name = "cs_top_tests",
    srcs = glob(["cs-top/src/**/*.rs"]),
    crate = ":cs_top_lib",
    deps = _TOOL_DEPS,
)

# ==============================================================================
# Tool Group Targets
# ==============================================================================
# Convenience targets for building multiple tools at once.

filegroup(
    name = "all",
    srcs = [
        ":cs_trace",
        ":cs_replay",
        ":cs_profile",
        ":cs_capgraph",
        ":cs_top",
    ],
    visibility = [
        "//:__pkg__",
        "//sdk:__pkg__",
    ],
)

filegroup(
    name = "all_libs",
    srcs = [
        ":cs_trace_lib",
        ":cs_replay_lib",
        ":cs_profile_lib",
        ":cs_capgraph_lib",
        ":cs_top_lib",
    ],
    visibility = [
        "//:__pkg__",
        "//sdk:__pkg__",
    ],
)

filegroup(
    name = "all_tests",
    srcs = [
        ":cs_trace_tests",
        ":cs_replay_tests",
        ":cs_profile_tests",
        ":cs_capgraph_tests",
        ":cs_top_tests",
    ],
    visibility = [
        "//:__pkg__",
        "//sdk:__pkg__",
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
    visibility = [
        "//:__pkg__",
        "//sdk:__pkg__",
    ],
)

# ==============================================================================
# VISIBILITY NOTES
# ==============================================================================
#
# Visibility Rules:
# - //:__pkg__      - Root workspace can reference tools
# - //sdk:__pkg__   - SDK layer can export tools
# - Prevents external dependencies on tools from docs, tests, etc.
#
# All tools are L3 (top layer) and can depend on:
# - L0 (kernel/):      capability_engine, ct_lifecycle, ipc_signals_exceptions
# - L1 (services/):    gpu_accelerator, semantic_memory, tool_registry_telemetry
# - L2 (runtime/):     framework_adapters, semantic_fs_agent_lifecycle
# - L3 (sdk/):         csci, libcognitive, other tools
#
# See docs/dependency_policy.md for comprehensive visibility policy.
