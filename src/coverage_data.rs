#[allow(unused)]
use log::{error, warn, info, debug, trace};

use rustc_demangle::demangle;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::utils::deser_from_str;

#[derive(Debug, Deserialize)]
pub(crate) struct CoverageMapping<'a> {
    #[serde(borrow)]
    pub files: Vec<FileCoverage<'a>>,

    #[serde(borrow)]
    pub functions: Vec<FunctionCoverage<'a>>,

    pub totals: FileCoverageSummary,
}

#[derive(Debug)]
pub(crate) struct FileBranch {
    pub line_start: i64,
    pub column_start: i64,
    pub line_end: i64,
    pub column_end: i64,
    pub execution_count: i64,
    pub false_execution_count: i64,
    pub file_id: i64,
    pub expanded_file_id: i64,
    pub region_kind: i64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FileCoverage<'a> {
    pub branches: Vec<FileBranch>,
    pub expansions: Vec<FileExpansion<'a>>,
    pub filename: &'a str,
    pub segments: Vec<FileSegment>,
    pub summary: FileCoverageSummary,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FileCoverageSummary {
    pub branches: Summary,
    pub functions: Summary,
    pub instantiations: Summary,
    pub lines: Summary,
    pub regions: Summary,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FileExpansion<'a> {
    #[serde(borrow)]
    pub filenames: Vec<&'a str>,
}

#[derive(Debug)]
pub(crate) struct FileSegment {
    pub line: i64,
    pub col: i64,
    pub count: i64,
    pub has_count: bool,
    pub is_region_entry: bool,
    pub is_gap_region: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FunctionCoverage<'a> {
    pub name: &'a str,

    pub count: i64,

    pub regions: Vec<Region>,

    #[serde(borrow)]
    pub filenames: Vec<&'a str>,
}

#[derive(Debug)]
pub(crate) struct Region {
    pub line_start: i64,
    pub column_start: i64,
    pub line_end: i64,
    pub column_end: i64,
    pub execution_count: i64,
    pub file_id: i64,
    pub expanded_file_id: i64,
    pub region_kind: i64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Summary {
    pub count: u64,
    pub covered: u64,
    #[serde(rename = "notcovered")]
    pub not_covered: Option<u64>,
    pub percent: f64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SummaryReport<'a> {
    #[serde(rename = "type")]
    pub report_type: &'a str,

    #[serde(deserialize_with = "deser_from_str")]
    pub version: semver::Version,

    pub data: Vec<CoverageMapping<'a>>,
}

impl From<[Value; 9]> for FileBranch {
    fn from(other: [Value; 9]) -> Self {
        Self {
            line_start: other[0].as_i64().unwrap(),
            column_start: other[1].as_i64().unwrap(),
            line_end: other[2].as_i64().unwrap(),
            column_end: other[3].as_i64().unwrap(),
            execution_count: other[4].as_i64().unwrap(),
            false_execution_count: other[5].as_i64().unwrap(),
            file_id: other[6].as_i64().unwrap(),
            expanded_file_id: other[7].as_i64().unwrap(),
            region_kind: other[8].as_i64().unwrap(),
        }
    }
}

impl<'de> Deserialize<'de> for FileBranch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = <[Value; 9]>::deserialize(deserializer)?;
        Ok(Self::from(data))
    }
}

impl From<[Value; 6]> for FileSegment {
    fn from(other: [Value; 6]) -> Self {
        Self {
            line: other[0].as_i64().unwrap(),
            col: other[1].as_i64().unwrap(),
            count: other[2].as_i64().unwrap(),
            has_count: other[3].as_bool().unwrap(),
            is_region_entry: other[4].as_bool().unwrap(),
            is_gap_region: other[5].as_bool().unwrap(),
        }
    }
}

impl<'de> Deserialize<'de> for FileSegment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = <[Value; 6]>::deserialize(deserializer)?;
        Ok(Self::from(data))
    }
}

impl<'a> FunctionCoverage<'a> {
    pub fn demangle(&self) -> String {
        format!("{:#}", demangle(self.name))
    }
}

impl From<[Value; 8]> for Region {
    fn from(other: [Value; 8]) -> Self {
        Self {
            line_start: other[0].as_i64().unwrap(),
            column_start: other[1].as_i64().unwrap(),
            line_end: other[2].as_i64().unwrap(),
            column_end: other[3].as_i64().unwrap(),
            execution_count: other[4].as_i64().unwrap(),
            file_id: other[5].as_i64().unwrap(),
            expanded_file_id: other[6].as_i64().unwrap(),
            region_kind: other[7].as_i64().unwrap(),
        }
    }
}

impl<'de> Deserialize<'de> for Region {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data = <[Value; 8]>::deserialize(deserializer)?;
        Ok(Self::from(data))
    }
}
