use std::{env, fs, process::ExitCode};

use appfw_runtime::{
    provider_certification::{
        area_report_json, external_api_area_report_json, provider_graduation_json,
        provider_matrix_json, provider_sdk_rules_json,
    },
    provider_keys::FrameworkProvider,
};

fn main() -> ExitCode {
    match run() {
        Ok(exit_code) => exit_code,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let mut provider = None;
    let mut log_file = None;
    let mut areas_only = false;
    let mut sdk_rules = false;
    let mut graduation = false;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--sdk-rules" => sdk_rules = true,
            "--graduation" => graduation = true,
            "--provider" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--provider requires a value".to_string())?;
                provider = Some(FrameworkProvider::parse_key(&value)?);
            }
            "--log-file" => {
                log_file = Some(
                    args.next()
                        .ok_or_else(|| "--log-file requires a path".to_string())?,
                );
            }
            "--areas" => areas_only = true,
            "--help" | "-h" => {
                print_usage();
                return Ok(ExitCode::SUCCESS);
            }
            _ => return Err(format!("unknown option: {arg}")),
        }
    }

    if sdk_rules {
        println!("{}", provider_sdk_rules_json());
        return Ok(ExitCode::SUCCESS);
    }

    if graduation {
        println!("{}", provider_graduation_json());
        return Ok(ExitCode::SUCCESS);
    }

    if areas_only {
        let provider = provider.ok_or_else(|| "--areas requires --provider".to_string())?;
        let (areas, failed) = if provider.is_external_api_provider() {
            external_api_area_report_json(provider)
        } else {
            let log_file = log_file.ok_or_else(|| {
                "--areas requires --log-file for database provider reports".to_string()
            })?;
            let log = fs::read_to_string(&log_file)
                .map_err(|err| format!("failed to read provider test log {log_file}: {err}"))?;
            area_report_json(provider, &log)
        };
        println!("{}", serde_json::Value::Array(areas));
        return Ok(if failed {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        });
    }

    println!("{}", provider_matrix_json(provider));
    Ok(ExitCode::SUCCESS)
}

fn print_usage() {
    let providers = FrameworkProvider::ALL
        .iter()
        .map(|provider| provider.key())
        .collect::<Vec<_>>()
        .join("|");
    println!(
        "Usage: provider_certification_export [--provider {providers}] [--log-file PATH --areas] [--sdk-rules] [--graduation]"
    );
}
