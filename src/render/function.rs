use std::error::Error as StdError;
use serde::Serialize;
use crate::FunctionCoverage;

use handlebars::Handlebars;
use std::path::Path;

#[derive(Serialize)]
struct Function {
    pub name: String,
    pub count: i64,
}

#[derive(Serialize)]
struct Context<'a> {
    package: Option<&'a str>,
    functions: Vec<Function>,
}

pub(crate) struct RenderFunction<'a> {
    func_coverage: &'a [&'a FunctionCoverage<'a>],
    package: Option<&'a str>,
    // input_path: &'a Path,
    handlebars: &'a Handlebars<'a>,
}

impl<'a> RenderFunction<'a> {
    pub fn new(func_coverage: &'a[&'a FunctionCoverage], package: Option<&'a str>, _input_path: &'a Path, handlebars: &'a Handlebars<'a>) -> Self {
        Self {
            func_coverage, package, handlebars
        }
    }

    pub fn render(&self) -> Result<String, Box<dyn StdError>> {
        let mut functions: Vec<Function> = self.func_coverage
            .iter()
            .map(|f| Function {
                name: f.demangle(),
                count: f.count,
            })
            .collect();
        functions.sort_by(|a, b| a.name.partial_cmp(&b.name).unwrap());

        let context = Context {
            package: self.package,
            functions
        };

        self.handlebars.render("functions", &context).map_err(|e| e.into())
    }
}
