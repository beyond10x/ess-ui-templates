//! `ess-ui`: a presentation page from a compiled ESS specification.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use ess_ui::page::{self, Inputs, Params, Presentation};
use ess_ui::Build;
use serde_json::Value;

#[derive(Parser)]
#[command(
    name = "ess-ui",
    version,
    about = "A presentation page from a compiled ESS specification: a Demo that plays the model, and a Model reference."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Write the self-contained HTML page.
    Page {
        #[command(flatten)]
        source: Source,
        /// The HTML file to write.
        #[arg(long)]
        out: PathBuf,
    },
    /// Write the page's data as JSON (`ess-ui-presentation/1`), for a React wrapper to render.
    Data {
        #[command(flatten)]
        source: Source,
        /// The JSON file to write.
        #[arg(long)]
        out: PathBuf,
    },
    /// Hold a page to the IR it was drawn from; exits 1 when a check fails.
    Check {
        #[command(flatten)]
        source: Source,
        /// The page to check.
        #[arg(long)]
        html: PathBuf,
    },
    /// Replay one authored scenario and write the final rows of every view as view fixtures.
    World {
        /// The compiled IR (`ess specify compile --format json`).
        #[arg(long)]
        ir: PathBuf,
        /// The specification directory.
        #[arg(long)]
        spec_dir: PathBuf,
        /// Authored scenarios (`ess verify conform author --out`).
        #[arg(long)]
        scenarios: PathBuf,
        /// The authored scenario to replay.
        #[arg(long)]
        scenario: String,
        /// The directory to write; replaced as a whole.
        #[arg(long)]
        out: PathBuf,
    },
    /// Write the JSON Schema of the data `ess-ui data` writes.
    Schema {
        /// The file to write.
        #[arg(long)]
        out: PathBuf,
    },
    /// Write the browser bundle as an ES module (`css`, `js`, `headJs`, `defs`) for the React wrapper.
    Assets {
        /// The file to write.
        #[arg(long)]
        out: PathBuf,
    },
    /// Write the embedded fonts as an ES module (`css`: the `@font-face` rules as `data:` URLs, with
    /// their OFL licences), which the React wrapper loads only when the host site serves none.
    Fonts {
        /// The file to write.
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum OnOff {
    On,
    Off,
}

#[derive(Args)]
struct Source {
    /// The compiled IR (`ess specify compile --format json --out`).
    #[arg(long)]
    ir: PathBuf,
    /// The specification directory: compiled to execute the model, and scanned for source lines.
    #[arg(long)]
    spec_dir: Option<PathBuf>,
    /// A synthesized suite (`ess verify conform synthesize --out`); replayed and playable.
    #[arg(long)]
    suite: Option<PathBuf>,
    /// Authored scenarios (`ess verify conform author --out`); the first is the Demo.
    #[arg(long)]
    scenarios: Option<PathBuf>,
    /// Where the authored scenario files are, for their `at:` times (default: the spec dir).
    #[arg(long)]
    scenario_dir: Option<PathBuf>,
    /// A semantic diff (`ess verify diff --format json`); added and changed declarations are badged.
    #[arg(long)]
    diff: Option<PathBuf>,
    /// The revision the diff compares against, for its label.
    #[arg(long)]
    diff_ref: Option<String>,
    /// The output of `ess verify conform synthesize`, for the overview.
    #[arg(long)]
    synth_log: Option<PathBuf>,
    /// A gate log whose last lines are a summary and `exit N`, for the overview.
    #[arg(long)]
    gate_log: Option<PathBuf>,
    /// The compiler version shown in the footer.
    #[arg(long)]
    compiler: Option<String>,
    /// A parameter file (JSON); flags override it. Default `<spec-dir>/ess-ui.json` when it exists.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Page title (default: the IR's system name).
    #[arg(long)]
    title: Option<String>,
    /// Web URL of the specification's repository.
    #[arg(long)]
    repo_url: Option<String>,
    /// The repository link's label (default "Repository").
    #[arg(long)]
    repo_label: Option<String>,
    /// The commit rendered (default: `git rev-parse HEAD` of the spec dir, marked dirty when it is).
    #[arg(long = "ref")]
    git_ref: Option<String>,
    /// Path of the specification inside the repository (default `.`).
    #[arg(long)]
    spec_root: Option<String>,
    /// Source-line URL template: `{repo}`, `{ref}`, `{path}`, `{line}`.
    #[arg(long)]
    source_url: Option<String>,
    /// Repository-tree URL template: `{repo}`, `{ref}`, `{path}`.
    #[arg(long)]
    tree_url: Option<String>,
    /// The entity whose lifecycle comes first.
    #[arg(long)]
    headline_entity: Option<String>,
    /// Comma-separated section ids, in the order wanted.
    #[arg(long, value_delimiter = ',')]
    section_order: Option<Vec<String>>,
    /// Land on the Demo tab (on), or hide it (off).
    #[arg(long, value_enum)]
    demo: Option<OnOff>,
    /// Open in presentation mode.
    #[arg(long, value_enum)]
    present: Option<OnOff>,
    /// The authored scenario the Demo plays (default: the first).
    #[arg(long)]
    demo_scenario: Option<String>,
}

fn read_json(p: &Path) -> Result<Value> {
    let t = std::fs::read_to_string(p).with_context(|| format!("reading {}", p.display()))?;
    serde_json::from_str(&t).with_context(|| format!("{} is not JSON", p.display()))
}

fn existing(p: Option<&PathBuf>) -> Option<&PathBuf> {
    p.filter(|p| p.exists())
}

impl Source {
    fn build(&self, out: Option<&Path>) -> Result<Build> {
        let ir_text = std::fs::read_to_string(&self.ir)
            .with_context(|| format!("reading {}", self.ir.display()))?;
        let cfg_path = self.config.clone().or_else(|| {
            self.spec_dir
                .as_ref()
                .map(|d| d.join("ess-ui.json"))
                .filter(|p| p.exists())
        });
        let mut p: Params = match &cfg_path {
            Some(path) => serde_json::from_value(read_json(path)?)
                .with_context(|| format!("{}: unknown or malformed parameter", path.display()))?,
            None => Params::default(),
        };
        macro_rules! over {
            ($($f:ident),*) => { $( if self.$f.is_some() { p.$f.clone_from(&self.$f); } )* };
        }
        over!(
            title,
            repo_url,
            repo_label,
            spec_root,
            source_url,
            tree_url,
            headline_entity,
            section_order,
            demo_scenario
        );
        if self.git_ref.is_some() {
            p.git_ref.clone_from(&self.git_ref);
        }
        if let Some(d) = self.demo {
            p.demo = Some(matches!(d, OnOff::On));
        }
        if let Some(d) = self.present {
            p.present = Some(matches!(d, OnOff::On));
        }
        let mut dirty = false;
        if p.git_ref.is_none() {
            if let Some(dir) = &self.spec_dir {
                let (sha, d) = page::git_ref(dir);
                p.git_ref = sha;
                dirty = d;
            }
        }
        let text = |f: Option<&PathBuf>| -> Result<Option<(String, String)>> {
            existing(f)
                .map(|p| Ok((p.display().to_string(), std::fs::read_to_string(p)?)))
                .transpose()
        };
        let json = |f: Option<&PathBuf>| -> Result<Option<(String, Value)>> {
            existing(f)
                .map(|p| Ok((p.display().to_string(), read_json(p)?)))
                .transpose()
        };
        let inputs = Inputs {
            spec_dir: self.spec_dir.clone(),
            scenario_dir: self.scenario_dir.clone(),
            suite: json(self.suite.as_ref())?,
            scenarios: json(self.scenarios.as_ref())?,
            diff: json(self.diff.as_ref())?.map(|(_, v)| v),
            diff_ref: self.diff_ref.clone(),
            synth_log: text(self.synth_log.as_ref())?,
            gate_log: text(self.gate_log.as_ref())?,
            compiler: self.compiler.clone(),
            out: out.map(Path::to_path_buf),
            dirty,
        };
        Ok(Build {
            ir_text,
            params: p,
            inputs,
        })
    }
}

fn write(out: &Path, text: &str) -> Result<()> {
    if let Some(dir) = out.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(out, text).with_context(|| format!("writing {}", out.display()))
}

fn report(p: &Presentation, what: &str, out: &Path, bytes: usize) {
    let e = &p.sim["engine"];
    println!(
        "wrote {what} {} ({bytes} bytes) from IR sha256 {}; {} command(s) executed by ESS's interpreter, {} by the page's extensions",
        out.display(),
        p.ir_sha256,
        e["ess"],
        e["page"]
    );
    for u in &p.unrendered {
        eprintln!("not rendered: {u}");
    }
}

fn assets_module() -> String {
    let s = |x: &str| serde_json::to_string(x).unwrap_or_default();
    format!(
        "// Generated by `ess-ui assets` from web/src; do not edit. `task check` fails when it is stale.\nexport const css = {};\nexport const js = {};\nexport const headJs = {};\nexport const defs = {};\n",
        s(&page::assets::css()),
        s(&page::assets::js()),
        s(page::assets::HEAD_JS.trim()),
        s(page::assets::DEFS.trim())
    )
}

fn fonts_module() -> String {
    format!(
        "// Generated by `ess-ui fonts` from web/fonts; do not edit. `task check` fails when it is stale.\nexport const css = {};\n",
        serde_json::to_string(&page::assets::fonts_css()).unwrap_or_default()
    )
}

fn run() -> Result<ExitCode> {
    match Cli::parse().command {
        Command::Page { source, out } => {
            let b = source.build(Some(&out))?;
            let (p, _) = ess_ui::present(&b)?;
            let html = page::html(&p);
            write(&out, &html)?;
            report(&p, "page", &out, html.len());
        }
        Command::Data { source, out } => {
            let b = source.build(None)?;
            let (p, _) = ess_ui::present(&b)?;
            let text = serde_json::to_string_pretty(&p)? + "\n";
            write(&out, &text)?;
            report(&p, "data", &out, text.len());
        }
        Command::Check { source, html } => {
            let b = source.build(Some(&html))?;
            let page_text = std::fs::read_to_string(&html)
                .with_context(|| format!("reading {}", html.display()))?;
            let mut rep =
                ess_ui::check::check(&b.ir_text, &page_text, b.inputs.spec_dir.as_deref());
            if b.inputs.spec_dir.is_some() {
                let (_, fresh) = ess_ui::present(&b)?;
                ess_ui::check::check_simulation(&mut rep, &page_text, &fresh);
            }
            print!("{}", rep.render());
            return Ok(if rep.passed() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            });
        }
        Command::World {
            ir,
            spec_dir,
            scenarios,
            scenario,
            out,
        } => {
            let ir_text = std::fs::read_to_string(&ir)?;
            let model = ess_ui::typed_model(&spec_dir, &ir_text)?;
            let interp = ess_ui::interp::Interp::new(&model);
            let suite = read_json(&scenarios)?;
            match ess_ui::world::write(&interp, &suite, &scenario, &out) {
                Ok(msg) => println!("world: {msg}"),
                Err(e) => {
                    eprintln!("world: {e:#}");
                    return Ok(ExitCode::from(1));
                }
            }
        }
        Command::Schema { out } => {
            let schema = schemars::schema_for!(Presentation);
            write(&out, &(serde_json::to_string_pretty(&schema)? + "\n"))?;
            println!("wrote {}", out.display());
        }
        Command::Assets { out } => {
            write(&out, &assets_module())?;
            println!("wrote {}", out.display());
        }
        Command::Fonts { out } => {
            write(&out, &fonts_module())?;
            println!("wrote {}", out.display());
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("ess-ui: {e:#}");
            ExitCode::from(2)
        }
    }
}
