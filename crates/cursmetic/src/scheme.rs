use std::collections::{HashMap, hash_map};
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

use anyhow::anyhow;
use cursmetic_recognizer::Cursor;
use strum::IntoEnumIterator;
use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
use windows::core::HRESULT;
use windows_registry::{CURRENT_USER, LOCAL_MACHINE, Transaction};

type CursorPathMap = HashMap<Cursor, PathBuf>;

const USER_CURSORS_SCHEMES: &str = "Control Panel\\Cursors\\Schemes";

const MACHINE_CURSORS_SCHEMES: &str =
    "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Control Panel\\Cursors\\Schemes";

#[derive(Debug, Clone)]
pub enum CursorSchemeName {
    User(String),
    Machine(String),
}

impl CursorSchemeName {
    pub fn exists(&self) -> anyhow::Result<bool> {
        let result = match self {
            Self::Machine(name) => LOCAL_MACHINE
                .open(MACHINE_CURSORS_SCHEMES)?
                .get_string(name),
            Self::User(name) => CURRENT_USER.open(USER_CURSORS_SCHEMES)?.get_string(name),
        };

        Ok(match result {
            Ok(_) => true,
            Err(r) if r.code() == HRESULT::from(ERROR_FILE_NOT_FOUND) => false,
            Err(r) => return Err(r.into()),
        })
    }
}

impl ToString for CursorSchemeName {
    fn to_string(&self) -> String {
        match self {
            Self::User(name) => name.clone(),
            Self::Machine(name) => name.clone(),
        }
    }
}

#[derive(Debug)]
pub enum CursorScheme {
    User {
        name: CursorSchemeName,
        paths: CursorPathMap,
    },
    Machine {
        name: CursorSchemeName,
        source: String,
        paths: CursorPathMap,
    },
}

impl Deref for CursorScheme {
    type Target = CursorPathMap;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::User { paths, .. } | Self::Machine { paths, .. } => paths,
        }
    }
}

impl DerefMut for CursorScheme {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::User { paths, .. } | Self::Machine { paths, .. } => paths,
        }
    }
}

impl IntoIterator for CursorScheme {
    type Item = (Cursor, PathBuf);
    type IntoIter = hash_map::IntoIter<Cursor, PathBuf>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::Machine { paths, .. } | Self::User { paths, .. } => paths.into_iter(),
        }
    }
}

impl CursorScheme {
    pub fn exists(&self) -> anyhow::Result<bool> {
        match self {
            Self::User { name, .. } | Self::Machine { name, .. } => name.exists(),
        }
    }

    pub fn name(&self) -> String {
        match self {
            Self::User { name, .. } | Self::Machine { name, .. } => name.to_string(),
        }
    }

    pub fn display_name(&self) -> anyhow::Result<String> {
        match self {
            Self::User { name, .. } => Ok(name.to_string()),
            Self::Machine { source, .. } => {
                let source = windows::core::HSTRING::from(source);
                let mut out = [0u16; 256];

                unsafe {
                    windows::Win32::UI::Shell::SHLoadIndirectString(
                        windows::core::PCWSTR(source.as_ptr()),
                        &mut out,
                        None,
                    )?;
                }

                let len = out.iter().position(|&c| c == 0).unwrap_or(out.len());
                let out = String::from_utf16_lossy(&out[..len]);

                Ok(out)
            }
        }
    }

    fn parse_reg_value(value: &str, name: String, expect_source: bool) -> anyhow::Result<Self> {
        let mut cursors: Vec<_> = Cursor::iter().filter(Cursor::is_standard).collect();

        cursors.sort();

        let (value, source) = match value.split_once('@') {
            Some((value, source)) => (value, Some(source)),
            None => {
                if expect_source {
                    return Err(anyhow!("not found @"));
                }

                (value, None)
            }
        };

        let mut paths: CursorPathMap = HashMap::new();

        for (i, v) in value.split(",").enumerate() {
            let Some(cursor) = cursors.get(i) else {
                continue;
            };

            paths.insert(cursor.clone(), PathBuf::from(v));
        }

        Ok(if let Some(source) = source {
            Self::Machine {
                name: CursorSchemeName::Machine(name),
                source: format!("@{source}"),
                paths,
            }
        } else {
            Self::User {
                name: CursorSchemeName::User(name),
                paths,
            }
        })
    }

    fn user_read(name: String) -> anyhow::Result<Self> {
        Self::parse_reg_value(
            &CURRENT_USER.open(USER_CURSORS_SCHEMES)?.get_string(&name)?,
            name,
            false,
        )
    }

    pub fn user_cursors() -> anyhow::Result<Vec<CursorSchemeName>> {
        Ok(CURRENT_USER
            .open(USER_CURSORS_SCHEMES)?
            .values()?
            .into_iter()
            .filter_map(|(k, _)| (!k.is_empty()).then_some(CursorSchemeName::User(k)))
            .collect())
    }

    fn machine_read(name: String) -> anyhow::Result<Self> {
        Self::parse_reg_value(
            &LOCAL_MACHINE
                .open(MACHINE_CURSORS_SCHEMES)?
                .get_string(&name)?,
            name,
            true,
        )
    }

    pub fn machine_cursors() -> anyhow::Result<Vec<CursorSchemeName>> {
        Ok(LOCAL_MACHINE
            .open(MACHINE_CURSORS_SCHEMES)?
            .values()?
            .into_iter()
            .filter_map(|(k, _)| (!k.is_empty()).then_some(CursorSchemeName::Machine(k)))
            .collect())
    }

    pub fn read(name: CursorSchemeName) -> anyhow::Result<Self> {
        match name {
            CursorSchemeName::User(name) => Self::user_read(name),
            CursorSchemeName::Machine(name) => Self::machine_read(name),
        }
    }

    fn stringify_reg_value(&self) -> String {
        let cursors = match self {
            Self::Machine { paths, .. } | Self::User { paths, .. } => paths,
        };
        let mut cursors: Vec<_> = Cursor::iter()
            .filter(Cursor::is_standard)
            .map(|c| (cursors.get(&c), c))
            .collect();

        cursors.sort_by(|a, b| a.1.cmp(&b.1));

        let mut cursors = cursors
            .iter()
            .map(|(v, _)| v.and_then(|v| v.to_str()).unwrap_or_default().to_string())
            .collect::<Vec<_>>();

        if let Self::Machine { source, .. } = self {
            cursors.push(source.clone());
        }

        cursors.join(",")
    }

    pub fn write(&self) -> anyhow::Result<()> {
        if self.name().is_empty() {
            return Err(anyhow!("Empty name"));
        }

        let s = self.stringify_reg_value();
        let tx = Transaction::new()?;
        let key = CURRENT_USER
            .options()
            .read()
            .write()
            .create()
            .transaction(&tx)
            .open(USER_CURSORS_SCHEMES)?;

        key.set_expand_string(self.name(), s)?;

        tx.commit()?;

        Ok(())
    }
}
