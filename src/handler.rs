use anyhow::{ Result, Context };
use std::{
    path::PathBuf, 
    fs
};
use log::{
    debug,
    info,
    error,
};
use notify::DebouncedEvent::{Write, Create, Chmod, Remove, self};
use crate::{
    config::Config, 
    bitburner::{
        BitburnerRequest, 
        write_file_to_server,
        delete_file_from_server,
    },
};

pub fn handle_event(config: &Config, event: &DebouncedEvent) -> Result<()> {
    debug!("event: {:?}", event);
    match event {
        Write(file) | Create(file) | Chmod(file) => {
            if is_valid_file(&file, &config) {
                info!("file change detected for file: {:?}", &file);
                let bitburner_request = build_bitburner_request(config, file, true)?;
                match write_file_to_server(config, &bitburner_request) {
                    Ok(res) => debug!("Response: {:?}", res),
                    Err(e) => error!("Network error: {:?}", e)
                }
            }
        },
        Remove(file) => {
            if is_valid_file(&file, &config) {
                info!("file deleted: {:?}", &file);
                let bitburner_request = build_bitburner_request(config, file, false)?;
                match delete_file_from_server(config, &bitburner_request) {
                    Ok(res) => debug!("Response: {:?}", res),
                    Err(e) => error!("Network error: {:?}", e)
                }
            }
        },
        unhandled_event => debug!("Unhandled event: {:?}", unhandled_event)
    }
    Ok(())
}

fn build_bitburner_request(config: &Config, path_buf: &PathBuf, include_code: bool) -> Result<BitburnerRequest> {
    let filename: String = extract_file_name(config, path_buf)?;
    let code: Option<String> = match include_code {
        true => {
            Some(
                base64::encode(
                    fs::read_to_string(
                        path_buf.as_path()
                    )
                    .expect("Unable to extract file contents")
                )
            )
        },
        false => None,
    };
    Ok(
        BitburnerRequest {
            filename,
            code,
        }
    )
}

// fn extract_file_name(config: &Config, path_buf: &PathBuf) -> Result<String> {
//     debug!("Building relative path from root: {:?} and file: {:?}", &config.directory, &path_buf);
//     let relative_path = path_buf.as_path().strip_prefix(Path::new(&config.directory))?;
//     debug!("Extracted relative path: ${:?}", &relative_path);
//     let filename = relative_path.to_str().unwrap().to_string();
//     debug!("Extracted filename: ${:?}", &filename);
//     Ok(filename)
// }

fn extract_file_name(config: &Config, path_buf: &PathBuf) -> Result<String> {
    path_buf.strip_prefix(&config.directory)
        .map(|path| path.to_str())?
        .map(|s| Ok(s.to_string()))
        .context("Unable to extract file name")?
}

fn is_valid_file(path_buf: &PathBuf, config: &Config) -> bool {
    path_buf.extension()
        .map(|ex| ex.to_str().unwrap_or("").to_string())
        .map(|s| config.valid_extensions.contains(&s))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use mockito::mock;
    use notify::DebouncedEvent;
    use super::*;

    #[test]
    fn assert_write_event_is_successful() {
        let _m = mock("PUT", "/")
            .with_status(200)
            .with_body("written")
            .create();
        let config = get_mock_config();
        let event = DebouncedEvent::Write(PathBuf::from(""));
        assert!(handle_event(&config, &event).is_ok());
    }

    #[test]
    fn assert_remove_event_is_successful() {
        let _m = mock("DELETE", "/")
            .with_status(200)
            .with_body("deleted")
            .create();
        let config = get_mock_config();
        let event = DebouncedEvent::Remove(PathBuf::from(""));
        assert!(handle_event(&config, &event).is_ok());
    }

    fn get_mock_config() -> Config {
        Config { bearer_token: String::from("token"), 
            port: String::from("9990"),
            url: String::from("url"),
            valid_extensions: vec![String::from("")],
            directory: String::from("") }
    }
}
