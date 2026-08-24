use std::collections::HashMap;
use std::io::{Stdout, stdout};
use std::path::PathBuf;
use std::process::exit;

use clap::Parser;
use crossterm::execute;
use crossterm::style::Stylize;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use cursmetic_recognizer::eval::WordEvaluator;
use cursmetic_recognizer::{Cursor, CursorMatch, CursorRecognizer};
use dialoguer::{Confirm, Input, Select};
use strum::IntoEnumIterator;
use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
use windows::core::HRESULT;
use windows_registry::{CURRENT_USER, Transaction};

#[derive(Debug, Parser)]
struct Cli {
    path: PathBuf,
}

#[derive(Debug)]
struct Suggestion {
    path: PathBuf,
    cursor: CursorMatch,
}

struct Screen {
    stdout: Stdout,
}

impl Screen {
    pub fn new() -> anyhow::Result<Self> {
        let mut stdout = stdout();

        execute!(stdout, EnterAlternateScreen)?;

        Ok(Self { stdout })
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        // TODO
        let _ = execute!(self.stdout, LeaveAlternateScreen);
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut suggestion: HashMap<Cursor, Vec<Suggestion>> = HashMap::new();

    if !cli.path.exists() {
        eprintln!("\"{}\" does not exist", cli.path.to_string_lossy());
        exit(1);
    }

    if !cli.path.is_dir() {
        eprintln!("\"{}\" is not a directory", cli.path.to_string_lossy());
        exit(1);
    }

    let recog = CursorRecognizer::new().evaluator(WordEvaluator::new()?);

    let root_name = cli
        .path
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string());
    let mut parents = vec![cli.path];

    while let Some(parent) = parents.pop() {
        for entry in parent.read_dir()?.filter_map(Result::ok) {
            let path = entry.path();

            if path.is_dir() {
                parents.push(path);

                continue;
            }

            let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
                continue;
            };

            if !matches!(ext, "ani" | "cur") {
                continue;
            }

            for m in recog.recognize(&path) {
                let suggestions = suggestion.entry(m.cursor.clone()).or_default();

                suggestions.push(Suggestion {
                    path: path.clone(),
                    cursor: m,
                });
            }
        }
    }

    let mut candidate: HashMap<Cursor, PathBuf> = HashMap::new();

    for cursor in Cursor::iter() {
        if let Some(suggestions) = suggestion.get_mut(&cursor) {
            suggestions.sort_by(|a, b| b.cursor.score.total_cmp(&a.cursor.score));
        }

        let Some(suggestions) = suggestion.get(&cursor) else {
            continue;
        };

        let mut best: Option<&Suggestion> = None;

        for s in suggestions {
            if s.cursor.score >= 0.3
                && best
                    .as_ref()
                    .is_none_or(|best| s.cursor.score > best.cursor.score)
            {
                best.replace(s);
            }
        }

        if let Some(best) = best {
            candidate.insert(cursor, best.path.clone());
        }
    }

    #[derive(Debug)]
    enum Item {
        Cursor {
            cursor: Cursor,
            display_path: Option<String>,
        },
        Separator,
        Cancel,
        Enter,
    }

    impl ToString for &Item {
        fn to_string(&self) -> String {
            match self {
                Item::Cursor {
                    display_path,
                    cursor,
                    ..
                } => format!(
                    "{} = {}",
                    cursor
                        .display_name()
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| cursor.to_string()),
                    display_path
                        .clone()
                        .unwrap_or_else(|| "<empty>".red().to_string())
                ),
                Item::Separator => "-".repeat(20),
                Item::Cancel => "Cancel".to_string(),
                Item::Enter => "Enter".to_string(),
            }
        }
    }

    let screen = Screen::new()?;

    loop {
        let items: Vec<_> = Cursor::iter()
            .filter(Cursor::is_standard)
            .filter_map(|cursor| {
                let Some(c) = candidate.get(&cursor) else {
                    return Some(Item::Cursor {
                        cursor,
                        display_path: None,
                    });
                };

                Some(Item::Cursor {
                    cursor,
                    display_path: Some(c.to_str()?.to_string()),
                })
            })
            .chain([Item::Separator, Item::Cancel, Item::Enter])
            .collect();
        let selection = Select::new().items(&items).interact()?;
        let selected = &items[selection];

        match selected {
            Item::Cancel => {
                drop(screen);
                // TODO
                println!("operation aborted");

                return Ok(());
            }
            Item::Separator => continue,
            Item::Enter => break,
            Item::Cursor { cursor, .. } => {
                enum Item {
                    Suggestion { path: PathBuf, score: f64 },
                    Separator,
                    Cancel,
                    EnterPath,
                    EmptyPath,
                }

                impl ToString for &Item {
                    fn to_string(&self) -> String {
                        match self {
                            Item::Suggestion { path, score } => {
                                format!("({}) {}", score, path.to_string_lossy())
                            }
                            Item::Separator => "-".repeat(20),
                            Item::Cancel => "Cancel".to_string(),
                            Item::EmptyPath => "Empty Path".to_string(),
                            Item::EnterPath => "Enter Path >".to_string(),
                        }
                    }
                }

                let _screen = Screen::new()?;

                loop {
                    let items: Vec<Item> = suggestion
                        .get(cursor)
                        .into_iter()
                        .flatten()
                        .map(|s| Item::Suggestion {
                            path: s.path.clone(),
                            score: s.cursor.score,
                        })
                        .chain([
                            Item::Separator,
                            Item::EnterPath,
                            Item::EmptyPath,
                            Item::Cancel,
                        ])
                        .collect();
                    let selected = Select::new().items(&items).interact()?;
                    let selected = &items[selected];

                    match selected {
                        Item::Separator => continue,
                        Item::Suggestion { path, .. } => {
                            candidate.insert(cursor.clone(), path.clone());

                            break;
                        }
                        Item::EmptyPath => {
                            candidate.remove(cursor);

                            break;
                        }
                        Item::EnterPath => {
                            let path: String = Input::new().interact_text()?;
                            let path = PathBuf::from(path);

                            candidate.insert(cursor.clone(), path);

                            break;
                        }
                        Item::Cancel => break,
                    }
                }
            }
        }
    }

    let theme_name: String = Input::new()
        .with_prompt("theme name")
        .default(root_name.unwrap_or_default())
        .interact()?;

    drop(screen);

    let mut cursors = Cursor::iter()
        .filter(Cursor::is_standard)
        .map(|c| (candidate.get(&c), c))
        .collect::<Vec<_>>();

    cursors.sort_by(|a, b| a.1.cmp(&b.1));

    let cursors = cursors
        .iter()
        .map(|(p, _)| p.and_then(|p| p.to_str()).unwrap_or_default().to_string())
        .collect::<Vec<_>>()
        .join(",");

    let exists = match CURRENT_USER
        .open("control panel\\cursors\\schemes")?
        .get_value(&theme_name)
    {
        Ok(_) => true,
        Err(r) if r.code() == HRESULT::from(ERROR_FILE_NOT_FOUND) => false,
        Err(r) => return Err(r.into()),
    };

    if exists
        && !Confirm::new()
            .default(false)
            .with_prompt("overwrite?")
            .interact()?
    {
        println!("operation aborted");

        return Ok(());
    }

    let tx = Transaction::new()?;
    let key = CURRENT_USER
        .options()
        .read()
        .write()
        .create()
        .transaction(&tx)
        .open("control panel\\cursors\\schemes")?;

    key.set_expand_string(theme_name, cursors)?;

    tx.commit()?;

    Ok(())
}
