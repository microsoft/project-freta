// Copyright (C) Microsoft Corporation. All rights reserved.

use serde::{Deserialize, Serialize};

/// Symbol representation
#[cfg_attr(feature = "proptest", derive(proptest_derive::Arbitrary))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Serialize, Clone, Eq, PartialEq, Deserialize)]
pub enum Symbol {
    /// Kernel symbol name
    Kernel(String),

    /// Kernel module symbol name
    Module(String, String),
}
