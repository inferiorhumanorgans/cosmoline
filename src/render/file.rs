use std::error::Error as StdError;
use std::fs::File;
use std::path::Path;
use std::io::{BufRead, BufReader};

use handlebars::Handlebars;
use serde::Serialize;
use log::{debug, trace};

use crate::{FileCoverage, utils};

pub(crate) struct RenderFile<'a> {
    file: &'a FileCoverage<'a>,
    package: Option<&'a str>,
    input_path: &'a Path,
    handlebars: &'a Handlebars<'a>
}

/// Collapsed segment with start and stop points
#[derive(Debug)]
struct Seg {
    pub start_col: i64,
    pub stop_col: i64,
    pub start_row: i64,
    pub stop_row: i64,
    pub count: i64,
}

/// Render context
#[derive(Serialize)]
struct Context<'a> {
    package: Option<&'a str>,
    filename: &'a str,
    contents: Vec<String>,
    max_line_len: usize,
    line_count_width: usize,
    lines_instrumented: u64,
    lines_hit: u64,
    lines_hit_percent: String,

    functions_instrumented: u64,
    functions_hit: u64,
    functions_hit_percent: String,
}

impl<'a> RenderFile<'a> {
    pub fn new(file: &'a FileCoverage<'a>, package: Option<&'a str>, input_path: &'a Path, handlebars: &'a Handlebars<'a>) -> Self {
        Self {
            file, package, input_path, handlebars
        }
    }

    pub fn render(&self) -> Result<String, Box<dyn StdError>> {
        use utils::InsertAtCharacter;

        debug!("Input: {:?}", self.input_path.join(self.file.filename));
        trace!("{:#?}\n\n", self.file);

        let input = File::open(self.input_path.join(self.file.filename))?;
        let input_reader = BufReader::new(input);
        let mut lines: Vec<String> = input_reader.lines().filter_map(Result::ok).collect();
        let max_line_len: usize = lines.iter().map(|l| l.len()).max().unwrap();
        let line_count_width: usize = ((lines.len() as f64).log10() + 1_f64).floor() as usize;
        let mut segments = vec![];

        for segment in self.file.segments.iter() {
            if segment.is_region_entry == true {
                segments.push(Seg {
                    start_col: segment.col,
                    stop_col: segment.col,
                    start_row: segment.line,
                    stop_row: segment.line,
                    count: segment.count,
                })
            } else {
                segments.last_mut().unwrap().stop_col = segment.col;
                segments.last_mut().unwrap().stop_row = segment.line;
                segments.last_mut().unwrap().count += segment.count;
            }
        }

        let segments: Vec<Seg> = segments.into_iter().rev().collect();

        for (seg_idx, segment) in segments.iter().enumerate() {
            if segment.start_row == segment.stop_row {
                let line_index = segment.start_row as usize - 1;
                lines[line_index].insert_at_char(segment.stop_col as usize, "{{ end_segment }}");
                lines[line_index].insert_at_char(
                    segment.start_col as usize,
                    &format!("{{{{ start_segment {} {} }}}}", seg_idx, segment.count),
                );
            } else {
                let start_idx = segment.start_row as usize - 1;
                lines[start_idx].push_str("{{ end_segment }}");
                lines[start_idx].insert_at_char(
                    segment.start_col as usize,
                    &format!("{{{{ start_segment {} {} }}}}", seg_idx, segment.count),
                );

                let stop_idx = segment.stop_row as usize - 1;
                lines[stop_idx].insert_at_char(segment.stop_col as usize, "{{ end_segment }}");

                lines[segment.stop_row as usize - 1].insert_at_char(
                    0,
                    &format!("{{{{ start_segment {} {} }}}}", seg_idx, segment.count),
                );
                for i in (segment.start_row + 1)..(segment.stop_row) {
                    lines[i as usize - 1].push_str("{{ end_segment }}");
                    lines[i as usize - 1].insert_at_char(
                        0,
                        &format!("{{{{ start_segment {} {} }}}}", seg_idx, segment.count),
                    );
                }
            }
            trace!("{:?}", segment)
        }

        for (i, line) in lines.iter().enumerate() {
            trace!("{:5}: {}", i, line)
        }

        let context = Context {
            package: self.package,
            filename: self.file.filename,
            contents: lines,
            max_line_len,
            line_count_width,
            lines_instrumented: self.file.summary.lines.count,
            lines_hit: self.file.summary.lines.covered,
            lines_hit_percent: format!("{:.2}", self.file.summary.lines.percent),
            functions_instrumented: self.file.summary.functions.count,
            functions_hit: self.file.summary.functions.covered,
            functions_hit_percent: format!("{:.2}", self.file.summary.functions.percent),
        };

        let re = regex::Regex::new(r#"\{\{ start_segment (\d+) (\d+) \}\}"#)?;
        let output = self.handlebars
            .render("file", &context)?
            .replace("{{ end_segment }}", "</span>");

        let output = re.replace_all(
            &output,
            r#"<span class='hit' title="${2} hits" data-count=${2} data-segment-index=${1}>"#,
        );

        Ok(output.to_string())

    }
}
