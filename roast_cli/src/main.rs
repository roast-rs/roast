use git2::{Config, Repository};
use log::{debug, error, info};
use roast::build;
use std::fs;
use std::path::Path;
use std::process::{exit, Command, Output};
use std::str::from_utf8;

use structopt::StructOpt;

include!(concat!(env!("OUT_DIR"), "/templates.rs"));

#[derive(Debug, StructOpt)]
#[structopt(name = "roast")]
struct Roast {
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: u8,
    #[structopt(subcommand)]
    cmd: RoastCommand,
}

#[derive(Debug, StructOpt)]
enum RoastCommand {
    #[structopt(
        name = "build",
        about = "Builds and generates the artifacts and source files"
    )]
    Build,
    #[structopt(name = "new", about = "Generates a new roast project")]
    New {
        #[structopt(help = "The name of the project")]
        name: String,
        #[structopt(
            name = "groupid",
            long = "groupid",
            short = "g",
            help = "Sets the group id for the java project"
        )]
        group_id: Option<String>,
        #[structopt(
            short = "f",
            long = "flavor",
            help = "Sets the java build flavor of the project",
            raw(possible_values = "&[\"maven\"]", case_insensitive = "true"),
            raw(default_value = "\"maven\"")
        )]
        flavor: String,
    },
}

fn main() {
    let args = Roast::from_args();

    // Always log info level as well (+1)
    loggerv::init_with_verbosity(u64::from(args.verbose) + 1).expect("Could not initialize the logger");

    match args.cmd {
        RoastCommand::Build => run_build(),
        RoastCommand::New {
            name,
            group_id,
            flavor,
        } => run_new(name, group_id, flavor),
    }
}

/// The `build` command is the workhorse of the project.
///
/// This command builds the rust project via `cargo build`,
/// then copies the compiled library into a place where
/// java can pick it up and then also copies the generated
/// java files into java's scope.
fn run_build() {
    info!("Building the rust project via `cargo build` (this may take a while)");

    match Command::new("cargo").arg("build").arg("-vv").output() {
        Ok(ref o) if o.status.success() => {
            debug!("`cargo build -vv` result {}", convert_output(&o))
        }
        Ok(e) => {
            error!("`cargo build -vv` failed! {}", convert_output(&e));
            exit(1);
        }
        Err(e) => {
            error!("`cargo build -vv` failed! {}", e);
            exit(1);
        }
    };
    let path = "roast.json";
    let spec = build::config_from_path(&path);
    debug!("Spec loaded from path {}:\n{:#?}", &path, &spec);

    info!("Copying build artifact into java scope");
    let extension = if cfg!(target_os = "windows") {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    };
    info!("{}", extension);
    let from = format!("{}/lib{}.{}", spec.bin_source(), spec.name(), extension);
    let to = format!("{}/lib{}.{}", spec.bin_target(), spec.name(), extension);
    debug!("Copying from {} to {}", from, to);
    match fs::copy(from, to) {
        Ok(_) => debug!("Copying completed"),
        Err(e) => {
            error!("Failed to copy artifacts: {}", e);
            exit(1);
        }
    };

    info!("Copying generated java sources into java scope");
    let from = spec.java_source();
    let to = spec.java_target();
    debug!("Copying from {} to {}", from, to);
    match Command::new("cp").arg("-r").arg(from).arg(to).output() {
        Ok(o) => debug!("`cp -r` result {}", convert_output(&o)),
        Err(e) => {
            error!("`cp -r` failed! {}", e);
            exit(1);
        }
    }

    info!("Build complete! Enjoy your roast!");
}

/// Takes a CLI output and formats it in a nice format for the CLI with
/// additional debug information if needed.
fn convert_output(o: &Output) -> String {
    format!(
        "(status: {})\n{}{}\n",
        o.status,
        from_utf8(o.stdout.as_ref())
            .expect("CLI output decoding failed because it is not valid UTF-8"),
        from_utf8(o.stderr.as_ref())
            .expect("CLI output decoding failed because it is not valid UTF-8"),
    )
}

/// The `new` command creates a new roast-bases project.
///
/// It basically grabs a template from its source and then
/// applies variable substitution to each file where needed
/// and writes the result in a folder provided.
///
/// Note that it also initializes a git project since that's
/// needed anyways mostly. We can add flags in the future to
/// customize further.
fn run_new(name: String, group_id: Option<String>, flavor: String) {
    let group_id = group_id.unwrap_or_else(|| String::from("rs.roast.gen"));

    info!("Creating project {}", name);

    let project_root = Path::new(&name);
    if project_root.exists() {
        error!(
            "Directory \"{}\" already exists, aborting!",
            project_root
                .to_str()
                .expect("Could not convert project root to string")
        );
        exit(1);
    }

    match fs::create_dir(&project_root) {
        Ok(_) => debug!("Project root directory created"),
        Err(e) => {
            error!("Error while creating directory {}", e);
            exit(1);
        }
    }

    debug!("Initializing git repository");
    let _repo = match Repository::init(&project_root) {
        Ok(repo) => repo,
        Err(e) => {
            error!("Error while initializing git {}", e);
            exit(1);
        }
    };

    let git_config = Config::open_default().expect("Could not open default git config");
    let user_name = git_config
        .get_string("user.name")
        .expect("Could not extract git user name");
    let user_email = git_config
        .get_string("user.email")
        .expect("Could not extract git user email");
    let author = format!("[\"{} <{}>\"]", user_name, user_email);

    let template_path = format!("templates/{}/", &flavor);

    let variables = vec![
        ("$NAME$", format!("\"{}\"", &name)),
        ("$AUTHORS$", author),
        ("$GROUPID$", group_id),
        ("$ARTIFACT$", name.clone()),
    ];

    for tpath in FILES.file_names() {
        if tpath.starts_with(&template_path) {
            let shortpath = tpath.replace(&template_path, "");
            let file_path = format!(
                "{}/{}",
                project_root
                    .to_str()
                    .expect("Could not convert project root to string"),
                &shortpath
            )
            .replace(".in", "");
            debug!("Creating file {}", &file_path);

            let mut content = String::from_utf8(
                FILES
                    .get(&tpath)
                    .expect("could not get template file")
                    .into_owned(),
            )
            .expect("Could not turn raw template file into utf8");
            for variable in &variables {
                content = content.replace(variable.0, &variable.1);
            }

            let filename = Path::new(&file_path)
                .file_name()
                .expect("could not extract filename");

            let dirpath =
                file_path.replace(filename.to_str().expect("could not convert filename"), "");
            fs::create_dir_all(dirpath).expect("could not create directory");
            fs::write(&file_path, content.as_bytes()).expect("could not write file");
        }
    }
}
