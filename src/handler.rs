use crate::{
    bitburner::{delete_file_from_server, write_file_to_server, BitburnerRequest},
    CONFIG,
};
use anyhow::{Context, Result};
use log::{debug, info};
use notify::event::{Event, EventKind};
use std::{fs, path::Path};

pub fn handle_event(event: &Event) -> Result<()> {
    if !event.clone().paths.into_iter().all(|it| is_valid_file(&it)) {
        debug!("ignoring event: {:#?}", &event);
        return Ok(());
    }
    let source = event
        .paths
        .first()
        .unwrap_or_else(|| panic!("unable to get source file for event: {:#?}", event));
    match &event.kind {
        EventKind::Create(_) => {
            info!("file created: {:#?}", &event);
            write_file_to_server(&build_bitburner_request(source, true)?)?;
        }
        EventKind::Modify(_) => {
            let destination = event.paths.last().unwrap_or_else(|| {
                panic!("unable to get destination file for event: {:#?}", event)
            });
            write_file_to_server(&build_bitburner_request(destination, true)?)?;
            if source == destination {
                info!("file {:#?} has been modified", &source);
            } else {
                info!("file {:#?} has been moved to {:#?}", &source, &destination);
                delete_file_from_server(&build_bitburner_request(source, false)?)?;
            }
        }
        EventKind::Remove(_) => {
            info!("file deleted: {:#?}", &event);
            delete_file_from_server(&build_bitburner_request(source, false)?)?;
        }
        unhandled_event => debug!("Unhandled event: {:#?}", unhandled_event),
    }
    Ok(())
}

#[allow(unused_variables)]
fn build_bitburner_request(path: &Path, include_code: bool) -> Result<BitburnerRequest> {
    #[cfg(test)]
    let include_code = false;
    let filename: String = extract_file_name(path)?;
    let code: Option<String> = match include_code {
        true => Some(base64::encode(
            fs::read_to_string(path).expect("Unable to extract file contents"),
        )),
        false => None,
    };
    Ok(BitburnerRequest { filename, code })
}

fn extract_file_name(path_buf: &Path) -> Result<String> {
    path_buf
        .strip_prefix(&CONFIG.directory)
        .map(|path| path.to_str())?
        .map(|s| Ok(s.to_string()))
        .context("Unable to extract file name")?
}

fn is_valid_file(path_buf: &Path) -> bool {
    path_buf
        .extension()
        .map(|ex| ex.to_str().unwrap_or("").to_string())
        .map(|s| CONFIG.valid_extensions.contains(&s))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::mock;
    use notify::event::{CreateKind, ModifyKind, RemoveKind};
    use std::path::PathBuf;

    #[test]
    fn assert_path_prefix_is_stripped() {
        assert_eq!(
            extract_file_name(&PathBuf::from("/one/two/three.txt")).unwrap(),
            String::from("three.txt")
        )
    }

    #[test]
    fn assert_valid_file() {
        assert!(is_valid_file(&PathBuf::from("test.js")));
    }

    #[test]
    fn assert_invalid_file() {
        assert!(!is_valid_file(&PathBuf::from("test.kt")));
    }

    #[test]
    fn assert_write_event_is_successful() {
        let _m1 = mock("PUT", "/")
            .with_status(200)
            .with_body("written")
            .create();
        let kind = EventKind::Create(CreateKind::Any);
        let event = Event::new(kind).add_path(PathBuf::from("/one/two/test.js"));
        assert!(handle_event(&event).is_ok());
    }

    #[test]
    fn assert_rename_event_is_successful() {
        let _m2 = mock("PUT", "/")
            .with_status(200)
            .with_body("written")
            .create();
        let _m3 = mock("DELETE", "/")
            .with_status(200)
            .with_body("deleted")
            .create();
        let kind = EventKind::Modify(ModifyKind::Any);
        let event = Event::new(kind)
            .add_path(PathBuf::from("/one/two/source.js"))
            .add_path(PathBuf::from("/one/two/destination.js"));
        assert!(handle_event(&event).is_ok());
    }

    #[test]
    fn assert_remove_event_is_successful() {
        let _m4 = mock("DELETE", "/")
            .with_status(200)
            .with_body("deleted")
            .create();
        let kind = EventKind::Remove(RemoveKind::Any);
        let event = Event::new(kind).add_path(PathBuf::from("/one/two/test.js"));
        assert!(handle_event(&event).is_ok());
    }
}
