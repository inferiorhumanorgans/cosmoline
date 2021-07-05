#![feature(destructuring_assignment)]

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[allow(unused)]
use log::{error, warn, info, debug, trace};

use clap::{crate_name, crate_version, App, Arg};
use env_logger::{Builder, Env};
use handlebars::Handlebars;
use serde::Serialize;

mod coverage_data;
use coverage_data::*;

mod utils {
    use serde::{de, Deserialize, Deserializer};
    use std::str::FromStr;

    /// Cheapie filename escape thing to flaten the paths
    /// so we don't actually need to create the whole hierarchy
    /// when generating the report.
    pub(crate) fn sanitize_filename(input: &str) -> String {
        format!("{}.html", input.replace("/", "_"))
    }

    /// Maps a percent to a color.  Will panic on negative values.
    pub(crate) fn color_for_percent<'a>(percent: f64) -> &'a str {
        match percent {
            i if i < 75.0 => "red",
            i if i >= 75.0 && i < 90.0 => "yellow",
            i if i >= 90.0 => "green",
            _ => unimplemented!(),
        }
    }

    /// Turns out String::insert_str will panic if we don't know where our character boundaries are e.g.
    /// multibyte characters (e.g. Cyrillic) mean the byte and character boundaries are in different locations.
    pub(crate) trait InsertAtCharacter {
        fn insert_at_char(&mut self, index: usize, s: &str);
    }

    impl InsertAtCharacter for String {
        fn insert_at_char(&mut self, index: usize, s: &str) {
            let char_indexes = self.char_indices().collect::<Vec<_>>();

            if index >= char_indexes.len() {
                self.push_str(s)
            } else {
                let index = char_indexes[index].0;
                if index >= self.len() {
                    self.push_str(s)
                } else {
                    if index > 0 {
                        self.insert_str(index - 1, s)
                    } else {
                        self.insert_str(index, s)
                    }
                }
            }
        }
    }

    // Ah boilerplate
    // https://github.com/serde-rs/json/issues/317
    pub(crate) fn deser_from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: FromStr,
        T::Err: std::fmt::Display,
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        T::from_str(&s).map_err(de::Error::custom)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use utils::InsertAtCharacter;

    #[cfg(debug_assertions)]
    Builder::from_env(Env::default().default_filter_or("info,cosmoline=debug"))
        .format_timestamp(None)
        .init();

    #[cfg(not(debug_assertions))]
    Builder::from_env(Env::default().default_filter_or("off"))
        .format_timestamp(None)
        .init();

    let matches = App::new(crate_name!())
        .version(crate_version!())
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output-directory")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("source-prefix")
                .short("p")
                .long("source-prefix")
                .takes_value(true)
        )
        .get_matches();

    let mut handlebars = Handlebars::new();

    let index_template_str = include_str!("../template/index.html.hbs");
    handlebars.register_template_string("index", index_template_str)?;

    let file_template_str = include_str!("../template/file.html.hbs");
    handlebars.register_template_string("file", file_template_str)?;

    let funcs_template_str = include_str!("../template/functions.html.hbs");
    handlebars.register_template_string("functions", funcs_template_str)?;

    let style_source = include_str!("../template/style.css");
    handlebars.register_template_string("style", style_source)?;

    let input_filename = matches.value_of("input").unwrap();
    let input_path = match matches.value_of("source-prefix") {
        Some(prefix) => Path::new(prefix),
        None => Path::new(input_filename).parent().unwrap()
    };
    let output_directory = matches.value_of("output").unwrap();
    let output_path = Path::new(output_directory);

    info!("Reading llvm JSON from: {}", input_filename);
    let mut file_contents = std::fs::read_to_string(input_filename)?;
    let summary_report: SummaryReport = serde_json::from_str(&mut file_contents)?;

    {
        match output_path.exists() {
            true => {
                let metadata = std::fs::metadata(output_directory)?;
                if metadata.file_type().is_dir() {
                    info!("Output directory exists at `{}'", output_path.display());
                } else {
                    error!("Non-directory exists at output `{}'", output_path.display());
                }
            }
            false => {
                // Make output directory
                std::fs::create_dir(output_path)?;
                info!(
                    "Created missing output directory `{}'",
                    output_path.display()
                );
            }
        }
    }

    info!("{} reports", summary_report.data.len());
    let file_coverage = summary_report.data[0]
        .files
        .iter()
        .filter(|x| x.filename.starts_with("src/"))
        .collect::<Vec<_>>();

    for file in file_coverage.iter() {
        debug!("Input: {:?}", input_path.join(file.filename));
        trace!("{:#?}\n\n", file);

        let input = File::open(input_path.join(file.filename))?;
        let input_reader = BufReader::new(input);
        let mut lines: Vec<String> = input_reader.lines().filter_map(Result::ok).collect();
        let max_line_len: usize = lines.iter().map(|l| l.len()).max().unwrap();
        let line_count_width: usize = ((lines.len() as f64).log10() + 1_f64).floor() as usize;
        let mut segments = vec![];

        #[derive(Debug)]
        struct Seg {
            pub start_col: i64,
            pub stop_col: i64,
            pub start_row: i64,
            pub stop_row: i64,
            pub count: i64,
        }

        for segment in file.segments.iter() {
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

        #[derive(Serialize)]
        struct Context<'a> {
            filename: &'a str,
            contents: Vec<String>,
            max_line_len: usize,
            line_count_width: usize,
        }
        let context = Context {
            filename: file.filename,
            contents: lines,
            max_line_len,
            line_count_width,
        };

        let re = regex::Regex::new(r#"\{\{ start_segment (\d+) (\d+) \}\}"#)?;
        let output = handlebars
            .render("file", &context)?
            .replace("{{ end_segment }}", "</span>");

        let output = re.replace_all(
            &output,
            r#"<span class='hit' title="${2} hits" data-count=${2} data-segment-index=${1}>"#,
        );

        let sanitized = utils::sanitize_filename(file.filename);
        std::fs::write(output_path.join(sanitized), &*output)?;
    }

    {
        #[derive(Serialize)]
        struct FileEntry<'a> {
            name: &'a str,
            link: String,
            pub lines_count: u64,
            pub lines_covered: u64,
            pub lines_percent: String,
            pub line_hit_class: &'a str,

            pub functions_count: u64,
            pub functions_covered: u64,
            pub functions_percent: String,
            pub function_hit_class: &'a str,
        }

        #[derive(Serialize)]
        struct Context<'a> {
            files: Vec<FileEntry<'a>>,
        }

        let context = Context {
            files: file_coverage
                .iter()
                .map(|f| FileEntry {
                    name: f.filename,
                    link: utils::sanitize_filename(f.filename),

                    lines_count: f.summary.lines.count,
                    lines_covered: f.summary.lines.covered,
                    lines_percent: format!("{:.1}", f.summary.lines.percent),
                    line_hit_class: utils::color_for_percent(f.summary.lines.percent),

                    functions_count: f.summary.functions.count,
                    functions_covered: f.summary.functions.covered,
                    functions_percent: format!("{:.1}", f.summary.functions.percent),
                    function_hit_class: utils::color_for_percent(f.summary.functions.percent),
                })
                .collect(),
        };

        std::fs::write(
            output_path.join("index.html"),
            handlebars.render("index", &context)?,
        )?;
    }

    // style.css
    {
        #[derive(Serialize)]
        struct Context {}

        let context = Context {};

        std::fs::write(
            output_path.join("style.css"),
            handlebars.render("style", &context)?,
        )?;
    }

    let func_coverage = summary_report.data[0]
        .functions
        .iter()
        .filter(|f| {
            f.filenames
                .iter()
                .filter(|x| x.starts_with("src/"))
                .collect::<Vec<_>>()
                .len()
                > 0
        })
        .collect::<Vec<_>>();

    {
        #[derive(Serialize)]
        struct Function {
            pub name: String,
            pub count: i64,
        }

        #[derive(Serialize)]
        struct Context {
            functions: Vec<Function>,
        }
        let mut functions: Vec<Function> = func_coverage
            .iter()
            .map(|f| Function {
                name: f.demangle(),
                count: f.count,
            })
            .collect();
        functions.sort_by(|a, b| a.name.partial_cmp(&b.name).unwrap());
        let context = Context { functions };
        std::fs::write(
            output_path.join("functions.html"),
            handlebars.render("functions", &context)?,
        )?;
    }

    Ok(())
}
