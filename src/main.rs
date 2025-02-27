mod config;
mod dependency_downloader;
mod janitor;
mod utils;

use std::env;
use std::process::exit;

use crate::config::{
    get_foundry_setup,
    read_config,
    remappings,
    Dependency,
};
use crate::dependency_downloader::{
    download_dependencies,
    unzip_dependencies,
    unzip_dependency,
};
use crate::janitor::{
    cleanup_after,
    healthcheck_dependencies,
};

const REMOTE_REPOSITORY: &str =
    "https://raw.githubusercontent.com/mario-eth/soldeer-versions/main/all_dependencies.toml";

#[derive(Debug)]
pub struct FOUNDRY {
    remappings: bool,
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let command: (String, String, String) = process_args(args).unwrap();

    // check the foundry setup, in case we have a foundry.toml, then the foundry.toml will be used for `sdependencies`
    let f_setup_vec: Vec<bool> = get_foundry_setup();
    let foundry_setup: FOUNDRY = FOUNDRY {
        remappings: f_setup_vec[0],
    };

    if command.0 == "install" && !command.1.is_empty() {
        let dependency_name: String = command.1.split('~').collect::<Vec<&str>>()[0].to_string();
        let dependency_version: String = command.1.split('~').collect::<Vec<&str>>()[1].to_string();
        let mut remote_url: String = REMOTE_REPOSITORY.to_string();
        if command.2.is_empty() {
            remote_url = command.2;
            let mut dependencies: Vec<Dependency> = Vec::new();
            dependencies.push(Dependency {
                name: dependency_name.clone(),
                version: dependency_version.clone(),
                url: remote_url.clone(),
            });
            dependency_url = remote_url.clone();
            if download_dependencies(&dependencies, true).await.is_err() {
                eprintln!("Error downloading dependencies");
                exit(500);
            }
        } else {
            match
                dependency_downloader::download_dependency_remote(
                    &dependency_name,
                    &dependency_version,
                    &remote_url
                ).await
            {
                Ok(url) => {
                    dependency_url = url;
                }
                Err(err) => {
                    eprintln!("Error downloading dependency: {:?}", err);
                    exit(500);
                }
            }
        }
        match unzip_dependency(&dependency_name, &dependency_version) {
            Ok(_) => {}
            Err(err) => {
                eprintln!("Error unzipping dependency: {:?}", err);
                exit(500);
            }
        }
        // TODO this is kinda junky written, need to refactor and a better TOML writer
        config::add_to_config(
            &dependency_name,
            &dependency_version,
            &dependency_url
        );

        match janitor::healthcheck_dependency(&dependency_name, &dependency_version) {
            Ok(_) => {}
            Err(err) => {
                eprintln!("Error health-checking dependency: {:?}", err);
                exit(500);
            }
        }
        match janitor::cleanup_dependency(&dependency_name, &dependency_version) {
            Ok(_) => {}
            Err(err) => {
                eprintln!("Error cleanup dependency: {:?}", err);
                exit(500);
            }
        }
        if foundry_setup.remappings {
            remappings();
        }
    } else if command.0 == "update" || (command.0 == "install" && command.1.is_empty()) {
        let dependencies: Vec<Dependency> = read_config(String::new(), &foundry_setup);
        if download_dependencies(&dependencies, true).await.is_err() {
            eprintln!("Error downloading dependencies");
            exit(500);
        }
        let result: Result<(), zip_extract::ZipExtractError> = unzip_dependencies(&dependencies);
        if result.is_err() {
            eprintln!("Error unzipping dependencies: {:?}", result.err().unwrap());
            exit(500);
        }
        let result: Result<(), janitor::MissingDependencies> =
            healthcheck_dependencies(&dependencies);
        if result.is_err() {
            eprintln!(
                "Error health-checking dependencies {:?}",
                result.err().unwrap().name
            );
            exit(500);
        }
        let result: Result<(), janitor::MissingDependencies> = cleanup_after(&dependencies);
        if result.is_err() {
            eprintln!(
                "Error cleanup dependencies {:?}",
                result.err().unwrap().name
            );
            exit(500);
        }
        if foundry_setup.remappings {
            remappings();
        }
    } else if command.0 == "help" {
        println!(
            "Usage: soldeer [command] [dependency] Example: dependency~version. the `~` is very important to differentiate between the name and the version that needs to be installed."
        );
        println!("Commands:");
        println!(
            "  install [dependency] (remote_url) - install a dependency, the `remote_url` is optional and defaults to the soldeer repository"
        );
        println!("  update - update all dependencies");
        println!("  help - show this help");
    }
}

fn process_args(args: Vec<String>) -> Result<(String, String, String), ()> {
    let command: String = String::from(&args[1]);
    let mut dependency: String = String::new();
    if args.len() > 2 {
        dependency = String::from(&args[2]);
    }
    if args.len() > 3 {
        return Ok((command, dependency, String::from(&args[3])));
    }
    Ok((command, dependency, String::new()))
}
