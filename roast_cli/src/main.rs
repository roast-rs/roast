#[macro_use]
extern crate clap;
extern crate roast;
#[macro_use]
extern crate log;
extern crate git2;
extern crate includedir;
extern crate loggerv;
extern crate phf;

use std::fs;
use std::process::{exit, Command, Output};

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use git2::{Config, Repository};
use roast::build;
use std::path::Path;
use std::str::from_utf8;

include!(concat!(env!("OUT_DIR"), "/templates.rs"));

fn main() {
    let matches = App::new("roast")
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .subcommand(
            SubCommand::with_name("build")
                .about("Builds and generates the artifacts and source files"),
        )
        .subcommand(
            SubCommand::with_name("new")
                .about("Generates a new roast project")
                .arg(
                    Arg::with_name("flavor")
                        .short("f")
                        .long("flavor")
                        .takes_value(true)
                        .possible_values(&["maven"])
                        .default_value("maven")
                        .help("Sets the java build flavor of the project"),
                )
                .arg(
                    Arg::with_name("groupid")
                        .short("g")
                        .long("groupid")
                        .takes_value(true)
                        .help("Sets the group id for the java project"),
                )
                .arg(
                    Arg::with_name("name")
                        .required(true)
                        .help("The name of the project"),
                ),
        )
        .get_matches();

    // Always log info level as well (+1)
    loggerv::init_with_verbosity(matches.occurrences_of("v") + 1)
        .expect("Could not initialize the logger");

    match matches.subcommand() {
        ("build", Some(bm)) => run_build(bm),
        ("new", Some(nm)) => run_new(nm),
        _ => panic!("Unknown command"),
    }
}

/// The `build` command is the workhorse of the project.
///
/// This command builds the rust project via `cargo build`,
/// then copies the compiled library into a place where
/// java can pick it up and then also copies the generated
/// java files into java's scope.
fn run_build(_m: &ArgMatches) {
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
    let from = format!("{}/lib{}.dylib", spec.bin_source(), spec.name());
    let to = format!("{}/lib{}.dylib", spec.bin_target(), spec.name());
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
fn run_new(m: &ArgMatches) {
    let name = m.value_of("name")
        .expect("Could not extract name from args!");
    let group_id = m.value_of("groupid").unwrap_or("rs.roast.gen");

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

    let flavor = m.value_of("flavor")
        .expect("Could not read flavor program argument");
    let template_path = format!("templates/{}/", &flavor);

    let variables = vec![
        ("$NAME$", format!("\"{}\"", name)),
        ("$AUTHORS$", author),
        ("$GROUPID$", group_id.into()),
        ("$ARTIFACT$", name.into()),
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
            ).replace(".in", "");
            debug!("Creating file {}", &file_path);

            let mut content = String::from_utf8(
                FILES
                    .get(&tpath)
                    .expect("could not get template file")
                    .into_owned(),
            ).expect("Could not turn raw template file into utf8");
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
