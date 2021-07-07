#![feature(destructuring_assignment)]

use std::path::Path;

#[allow(unused)]
use log::{error, warn, info, debug, trace};

use clap::{crate_name, crate_version, App, Arg};
use env_logger::{Builder, Env};
use handlebars::{self as hbs, Handlebars};
use serde::Serialize;

mod coverage_data;
use coverage_data::*;

mod render;
mod utils;

fn setup_handlebars<'a>() -> Result<Handlebars<'a>, Box<dyn std::error::Error>> {
    let mut handlebars = Handlebars::new();

    handlebars.register_helper("strftime",
      Box::new(|h: &hbs::Helper, _r: &hbs::Handlebars, _: &hbs::Context, _rc: &mut hbs::RenderContext, out: &mut dyn hbs::Output| -> hbs::HelperResult {
          let time_arg : &str = h.param(0).ok_or(hbs::RenderError::new("time param not found"))?.value().as_str().unwrap();
          let format_arg : &str = h.param(1).ok_or(hbs::RenderError::new("format param not found"))?.value().as_str().unwrap();

          let time = chrono::DateTime::parse_from_rfc3339(time_arg).map_err(|e| hbs::RenderError::new(e.to_string()))?;

          out.write(
            &format!("{}", time.format(format_arg))
          ).map_err(|e| hbs::RenderError::new(e.to_string()))
      }));

    let index_template_str = include_str!("../template/index.html.hbs");
    handlebars.register_template_string("index", index_template_str)?;

    let file_template_str = include_str!("../template/file.html.hbs");
    handlebars.register_template_string("file", file_template_str)?;

    let funcs_template_str = include_str!("../template/functions.html.hbs");
    handlebars.register_template_string("functions", funcs_template_str)?;

    let style_source = include_str!("../template/style.css");
    handlebars.register_template_string("style", style_source)?;

    Ok(handlebars)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
        .arg(
            Arg::with_name("package-name")
                .short("n")
                .long("package-name")
                .takes_value(true)
        )
        .get_matches();

    let handlebars = setup_handlebars()?;

    let input_filename = matches.value_of("input").unwrap();
    let input_path = match matches.value_of("source-prefix") {
        Some(prefix) => Path::new(prefix),
        None => Path::new(input_filename).parent().unwrap()
    };

    let output_directory = matches.value_of("output").unwrap();
    let output_path = Path::new(output_directory);

    let package = matches.value_of("package-name");

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
        use render::RenderFile;
        let render = RenderFile::new(file, package, input_path, &handlebars);
        let output = render.render()?;

        let sanitized = utils::sanitize_filename(file.filename);
        std::fs::write(output_path.join(sanitized), &*output)?;
    }

    {
        use render::RenderIndex;
        let render = RenderIndex::new(&file_coverage, &summary_report.data[0].totals, package, input_path, &handlebars);

        std::fs::write(
            output_path.join("index.html"),
            render.render()?,
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

    println!("Report written to {}/index.html", output_path.display());

    Ok(())
}
