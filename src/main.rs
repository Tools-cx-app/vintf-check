use std::{fs, process::Command};

use anyhow::Result;
use quick_xml::{Reader, events::Event};

fn get_kernel_level() -> (String, String) {
    let out = Command::new("uname").arg("-r").output().unwrap();
    let full = String::from_utf8(out.stdout).unwrap();
    let full = full.trim_end(); // 5.10.245-localhost-Hutao

    let mut parts = full.split('.');
    let major = parts.next().unwrap(); // "5"
    let minor = parts.next().unwrap(); // "10"
    (major.to_owned(), minor.to_owned())
}

fn main() -> Result<()> {
    let output = Command::new("zcat").arg("/proc/config.gz").output()?.stdout;
    let stdout = String::from_utf8_lossy(&output);
    let (level, sub_level) = get_kernel_level();
    let version = format!("{level}.{sub_level}");
    let mut inside_target_kernel = false;

    for i in stdout.clone().lines() {
        let mid = i.find('=');

        if mid.is_none() {
            continue;
        }

        let (k, v) = i.split_at(mid.unwrap());

        for dir in fs::read_dir("/system/etc/vintf/")? {
            let dir = dir?;

            if dir.file_type()?.is_dir() {
                continue;
            }

            let content = fs::read_to_string(dir.path())?;

            let mut reader = Reader::from_str(&content);

            let mut buf = Vec::new();
            let mut current_key: Option<String> = None;

            loop {
                match reader.read_event_into(&mut buf)? {
                    Event::Start(e) => {
                        let name = e.name();
                        let name = name.as_ref();
                        if name == b"kernel" {
                            let ver_attr = e
                                .attributes()
                                .find(|a| a.as_ref().unwrap().key.as_ref() == b"version");
                            if let Some(Ok(attr)) = ver_attr {
                                let ver = attr.unescape_value()?.into_owned();
                                inside_target_kernel = ver == version.as_str();
                            }
                            continue;
                        }

                        if !inside_target_kernel {
                            continue;
                        }

                        if name == b"key" {
                            current_key = Some(reader.read_text(e.name())?.to_string());
                        } else if name == b"value" {
                            if let Some(xml_k) = current_key.take() {
                                let xml_v = reader.read_text(e.name())?;
                                if k.to_string() == xml_k && v.to_string() != xml_v {
                                    println!("This option({xml_k}) should be {xml_v}");
                                }
                            }
                        }
                    }
                    Event::Eof => break,
                    _ => {}
                }
                buf.clear();
            }
        }
        println!("No conflict found");
    }
    Ok(())
}
