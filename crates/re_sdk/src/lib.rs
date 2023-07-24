//! The Rerun SDK
//!
//! ## Feature flags
#![doc = document_features::document_features!()]
//!

#![warn(missing_docs)] // Let's keep the this crate well-documented!

// ----------------
// Private modules:

mod global;
mod log_sink;
mod msg_sender;
mod recording_stream;

// -------------
// Public items:

pub use self::msg_sender::{MsgSender, MsgSenderError};
pub use self::recording_stream::{RecordingStream, RecordingStreamBuilder};

pub use re_sdk_comms::default_server_addr;

pub use re_log_types::{ApplicationId, EntityPath, LegacyComponent, StoreId, StoreKind};
pub use re_types::ComponentName;

pub use global::cleanup_if_forked_child;

#[cfg(not(target_arch = "wasm32"))]
impl crate::sink::LogSink for re_log_encoding::FileSink {
    fn send(&self, msg: re_log_types::LogMsg) {
        re_log_encoding::FileSink::send(self, msg);
    }

    #[inline]
    fn flush_blocking(&self) {}
}

// ---------------
// Public modules:

#[cfg(feature = "demo")]
pub mod demo_util;

/// Different destinations for log messages.
///
/// This is how you select whether the log stream ends up
/// sent over TCP, written to file, etc.
pub mod sink {
    pub use crate::log_sink::{BufferedSink, LogSink, MemorySink, MemorySinkStorage, TcpSink};

    #[cfg(not(target_arch = "wasm32"))]
    pub use re_log_encoding::{FileSink, FileSinkError};
}

/// Things directly related to logging.
pub mod log {
    pub use re_log_types::{
        DataCell, DataRow, DataTable, DataTableBatcher, DataTableBatcherConfig, LogMsg, PathOp,
        RowId, TableId,
    };
}

/// Time-related types.
pub mod time {
    pub use re_log_types::{Time, TimeInt, TimePoint, TimeType, Timeline};
}

/// These are the different _components_ you can log.
///
/// They all implement the [`Component`][`re_types::Component`] trait,
/// and can be used in [`MsgSender::with_component`].
pub mod components {
    pub use re_components::{
        AnnotationContext, AnnotationInfo, Arrow3D, Box3D, ClassDescription, ClassId, ColorRGBA,
        DisconnectedSpace, DrawOrder, EncodedMesh3D, InstanceKey, KeypointId, Label, LineStrip2D,
        LineStrip3D, Mat3x3, Mesh3D, MeshFormat, MeshId, Pinhole, Point2D, Point3D, Quaternion,
        Radius, RawMesh3D, Rect2D, Scalar, ScalarPlotProps, Tensor, TensorData, TensorDataMeaning,
        TensorDimension, TensorId, TextEntry, Transform3D, Vec2D, Vec3D, Vec4D, ViewCoordinates,
    };
}

/// Transform helpers, for use with [`components::Transform3D`].
pub mod transform {
    pub use re_components::{
        Angle, Rotation3D, RotationAxisAngle, Scale3D, Transform3DRepr, TranslationAndMat3,
        TranslationRotationScale3D,
    };
}

/// Coordinate system helpers, for use with [`components::ViewCoordinates`].
pub mod coordinates {
    pub use re_components::coordinates::{Axis3, Handedness, Sign, SignedAxis3};
}

/// Experimental APIs, to try out upcoming Rerun features.
pub mod experimental {
    pub use re_types::{archetypes, components, datatypes, Archetype, Component, Datatype};
}

/// Re-exports of other crates.
pub mod external {
    pub use re_log;
    pub use re_log_types;
    pub use re_memory;
    pub use re_sdk_comms;
    pub use re_types;

    #[cfg(feature = "glam")]
    pub use re_components::external::glam;

    #[cfg(feature = "image")]
    pub use re_components::external::image;
}

// -----
// Misc:

/// The version of the Rerun SDK.
pub fn build_info() -> re_build_info::BuildInfo {
    re_build_info::build_info!()
}

const RERUN_ENV_VAR: &str = "RERUN";

/// Helper to get the value of the `RERUN` environment variable.
fn get_rerun_env() -> Option<bool> {
    std::env::var(RERUN_ENV_VAR)
        .ok()
        .and_then(|s| match s.to_lowercase().as_str() {
            "0" | "false" | "off" => Some(false),
            "1" | "true" | "on" => Some(true),
            _ => {
                re_log::warn!(
                    "Invalid value for environment variable {RERUN_ENV_VAR}={s:?}. Expected 'on' or 'off'. It will be ignored"
                );
                None
            }
        })
}

/// Checks the `RERUN` environment variable. If not found, returns the argument.
///
/// Also adds some helpful logging.
pub fn decide_logging_enabled(default_enabled: bool) -> bool {
    // We use `info_once` so that we can call this function
    // multiple times without spamming the log.
    match get_rerun_env() {
        Some(true) => {
            re_log::info_once!(
                "Rerun Logging is enabled by the '{RERUN_ENV_VAR}' environment variable."
            );
            true
        }
        Some(false) => {
            re_log::info_once!(
                "Rerun Logging is disabled by the '{RERUN_ENV_VAR}' environment variable."
            );
            false
        }
        None => {
            if !default_enabled {
                re_log::info_once!(
                    "Rerun Logging has been disabled. Turn it on with the '{RERUN_ENV_VAR}' environment variable."
                );
            }
            default_enabled
        }
    }
}

// ----------------------------------------------------------------------------

/// Creates a new [`re_log_types::StoreInfo`] which can be used with [`RecordingStream::new`].
#[track_caller] // track_caller so that we can see if we are being called from an official example.
pub fn new_store_info(
    application_id: impl Into<re_log_types::ApplicationId>,
) -> re_log_types::StoreInfo {
    re_log_types::StoreInfo {
        application_id: application_id.into(),
        store_id: StoreId::random(StoreKind::Recording),
        is_official_example: called_from_official_rust_example(),
        started: re_log_types::Time::now(),
        store_source: re_log_types::StoreSource::RustSdk {
            rustc_version: env!("RE_BUILD_RUSTC_VERSION").into(),
            llvm_version: env!("RE_BUILD_LLVM_VERSION").into(),
        },
        store_kind: re_log_types::StoreKind::Recording,
    }
}

#[track_caller]
fn called_from_official_rust_example() -> bool {
    // The sentinel file we use to identify the official examples directory.
    const SENTINEL_FILENAME: &str = ".rerun_examples";
    let caller = core::panic::Location::caller();
    let mut path = std::path::PathBuf::from(caller.file());
    let mut is_official_example = false;
    for _ in 0..4 {
        path.pop(); // first iteration is always a file path in our examples
        if path.join(SENTINEL_FILENAME).exists() {
            is_official_example = true;
        }
    }
    is_official_example
}

// ---------------------------------------------------------------------------

/// Wrapper around puffin profiler on native, no-op on weasm
#[doc(hidden)]
#[macro_export]
macro_rules! profile_function {
    ($($arg: tt)*) => {
        #[cfg(not(target_arch = "wasm32"))]
        puffin::profile_function!($($arg)*);
    };
}

/// Wrapper around puffin profiler on native, no-op on weasm
#[doc(hidden)]
#[macro_export]
macro_rules! profile_scope {
    ($($arg: tt)*) => {
        #[cfg(not(target_arch = "wasm32"))]
        puffin::profile_scope!($($arg)*);
    };
}
