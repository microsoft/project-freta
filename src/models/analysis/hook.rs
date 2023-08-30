// Copyright (C) Microsoft Corporation. All rights reserved.

use crate::models::analysis::{memory::VirtualAddress, symbols::Symbol};
use serde::{Deserialize, Serialize};

/// An issue found in the analysis of a Freta snapshot
#[cfg_attr(feature = "proptest", derive(proptest_derive::Arbitrary))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Check {
    /// Basic information about the issue
    pub issue: String,

    /// Detailed information about the issue
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,

    /// If the issue was a hooked function, information about the hook
    #[serde(flatten)]
    pub hook: Option<Hook>,

    /// The virtual memory address related to the issue
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<VirtualAddress>,

    /// The symbol related to the issue
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "proptest", proptest(value = "None"))]
    pub symbol: Option<Symbol>,

    /// Process IDs involved in the issue
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub pids: Vec<u64>,

    /// Paths involved in the issue
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub paths: Vec<String>,

    /// Export path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exported_path: Option<String>,
}

/// Information about a hooked function
#[cfg_attr(feature = "proptest", derive(proptest_derive::Arbitrary))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, PartialEq, Eq, Serialize, Clone, Default, Deserialize)]
pub struct Hook {
    /// Address of the hooked function
    pub addr: VirtualAddress,

    /// type of hook
    pub hook_type: String,

    /// disassembly of the hooked function
    pub disassembly: String,

    /// calculated address for the destination of the hook
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_addr: Option<VirtualAddress>,

    /// disassembly of the destination for the hooked function
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_disassembly: Option<String>,

    /// symbol name for the destination for the hooked function if known
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "proptest", proptest(value = "None"))]
    pub target_module: Option<Symbol>,
}
