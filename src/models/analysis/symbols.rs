// Copyright (C) Microsoft Corporation. All rights reserved.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Symbol representation
#[derive(Debug, Serialize, JsonSchema, Clone, Eq, PartialEq, Deserialize)]
pub enum Symbol {
    /// Kernel symbol name
    Kernel(String),

    /// Kernel module symbol name
    Module(String, String),
}
