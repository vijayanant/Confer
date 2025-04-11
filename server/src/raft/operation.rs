use crate::proto::confer::v1::{ConfigPath, ConfigValue};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Operation {
    Set {
        path: ConfigPath,
        value: ConfigValue,
    },
    Remove {
        path: ConfigPath,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OperationResponse {
    Get { value: Option<ConfigValue> },
    Success,
    Failure,
}

// The `Operation` and `OperationResponse` enum automatically implements the `openraft::AppData` trait due to:
// 1. `#[derive(Serialize, Deserialize)]`: This derives `serde::Serialize` and `serde::Deserialize`,
//    satisfying the `OptionalSerde` trait bound in `openraft`'s blanket `AppData` implementation.
// 2. `'static` lifetime: The enum owns all its data and has no non-`'static` references.
// 3. `Send` and `Sync`: By default, the enum is `Send` and `Sync` because it contains
//    types that are also `Send` and `Sync`.
//
// Therefore, the blanket `AppData` implementation in `openraft` automatically
// applies to `Operation`, and no explicit `impl AppData for Operation {}` is needed.

//impl openraft::AppData for Operation {}
//impl openraft::AppDataResponse for OperationResponse {}
