use crate::LicenseCommands;
use anyhow::{Context, Result};
use license_client::LicenseClient;
use std::process::ExitCode;

pub fn run(command: LicenseCommands) -> Result<ExitCode> {
    let client = LicenseClient::from_env().context("Failed to initialize license client")?;

    match command {
        LicenseCommands::Activate { key } => {
            let key = key.trim();
            let result = client.activate(key).context("Activation failed")?;
            println!("License activated: {}", result.status.license_key);
            println!("Status: {}", result.status.status);
            println!("Device slots: {}", result.status.max_devices);
            if let Some(period_end) = result.status.period_end {
                println!("Period end (unix): {}", period_end);
            }
            Ok(ExitCode::from(0))
        }
        LicenseCommands::Status { key, refresh } => {
            let key = key
                .as_deref()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty());
            let status = if refresh {
                client.status_remote(key).context("Status request failed")?
            } else {
                match client.load_local_state()? {
                    Some(state) => {
                        println!("Local token: {}", state.token.payload.license_key);
                        println!("Status: {}", state.token.payload.status);
                        println!("Expires at (unix): {}", state.token.payload.expires_at);
                        if let Some(period_end) = state.token.payload.period_end {
                            println!("Period end (unix): {}", period_end);
                        }
                        return Ok(ExitCode::from(0));
                    }
                    None => client.status_remote(key).context("Status request failed")?,
                }
            };
            println!("License: {}", status.license_key);
            println!("Status: {}", status.status);
            println!(
                "Devices: {} / {}",
                status.activations.len(),
                status.max_devices
            );
            if let Some(trial_end) = status.trial_end {
                println!("Trial end (unix): {}", trial_end);
            }
            if let Some(period_end) = status.period_end {
                println!("Period end (unix): {}", period_end);
            }
            Ok(ExitCode::from(0))
        }
        LicenseCommands::Deactivate { key } => {
            let key = key
                .as_deref()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty());
            client.deactivate(key).context("Deactivation failed")?;
            println!("License deactivated for this device.");
            Ok(ExitCode::from(0))
        }
    }
}
