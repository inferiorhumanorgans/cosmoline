use std::error::Error as StdError;
use std::fs::metadata;

use chrono::{DateTime, offset::Local};
use serde::Serialize;

use crate::{FileCoverage, FileCoverageSummary, utils};
use handlebars::Handlebars;
use std::path::Path;

pub(crate) struct RenderIndex<'a> {
    files: &'a Vec<&'a FileCoverage<'a>>,
    totals: &'a FileCoverageSummary,
    package: Option<&'a str>,
    input_path: &'a Path,
    handlebars: &'a Handlebars<'a>
}

#[derive(Serialize)]
struct FileEntry<'a> {
    name: &'a str,
    link: String,
    pub lines_count: u64,
    pub lines_covered: u64,
    pub lines_percent: String,
    pub lines_percent_n: String,
    pub lines_percent_d: String,
    pub line_hit_class: &'a str,

    pub functions_count: u64,
    pub functions_covered: u64,
    pub functions_percent: String,
    pub functions_percent_n: String,
    pub functions_percent_d: String,
    pub function_hit_class: &'a str,
}

#[derive(Serialize)]
struct Context<'a> {
    title: String,
    input_mtime: String,
    total_line_hit_rate: String,
    total_func_hit_rate: String,
    files: Vec<FileEntry<'a>>,
}

impl<'a> RenderIndex<'a> {
    pub fn new(files: &'a Vec<&FileCoverage<'a>>, totals: &'a FileCoverageSummary, package: Option<&'a str>, input_path: &'a Path, handlebars: &'a Handlebars<'a>) -> Self {
        Self {
            files, totals, package, input_path, handlebars
        }
    }

    pub fn render(&self) -> Result<String, Box<dyn StdError>> {

        let input_mtime : DateTime<Local> = metadata(self.input_path)?.modified()?.into();

        let context = Context {
            title: match self.package {
                Some(package) => format!("Code Coverage for {}", package),
                None => format!("Code Coverage Report")
            },
            input_mtime: input_mtime.to_rfc3339(),
            total_line_hit_rate: format!("{:.1}", self.totals.lines.percent),
            total_func_hit_rate: format!("{:.1}", self.totals.functions.percent),
            files: self.files
                .iter()
                .map(|f| {
                    let lines_percent = format!("{:.1}", f.summary.lines.percent);
                    let lines_percent_vec = lines_percent.splitn(2, ".").into_iter().collect::<Vec<_>>();

                    let functions_percent = format!("{:.1}", f.summary.functions.percent);
                    let funcs_percent_vec = functions_percent.splitn(2, ".").into_iter().collect::<Vec<_>>();

                    FileEntry {
                        name: f.filename,
                        link: utils::sanitize_filename(f.filename),

                        lines_count: f.summary.lines.count,
                        lines_covered: f.summary.lines.covered,
                        lines_percent_n: lines_percent_vec[0].into(),
                        lines_percent_d: lines_percent_vec[1].into(),
                        lines_percent,
                        line_hit_class: utils::color_for_percent(f.summary.lines.percent),

                        functions_count: f.summary.functions.count,
                        functions_covered: f.summary.functions.covered,
                        functions_percent_n: funcs_percent_vec[0].into(),
                        functions_percent_d: funcs_percent_vec[1].into(),
                        functions_percent,
                        function_hit_class: utils::color_for_percent(f.summary.functions.percent),
                    }
                })
                .collect(),
        };

        self.handlebars.render("index", &context).map_err(|e| e.into())
    }
}
