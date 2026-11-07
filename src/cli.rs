// command line options: --demo, --file PATH, --decks PATH, --help
use std::path::PathBuf;

#[derive(Debug, Default, PartialEq)]
pub struct Args {
    pub demo: bool,
    pub file: Option<PathBuf>,
    pub decks: Option<PathBuf>,
    pub help: bool,
}

pub fn parse(args: &[String]) -> Result<Args, String> {
    let mut out = Args::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--demo" => out.demo = true,
            "-h" | "--help" => out.help = true,
            "--decks" => {
                let path = it
                    .next()
                    .ok_or_else(|| "--decks needs a path".to_string())?;
                out.decks = Some(PathBuf::from(path));
            }
            "-f" | "--file" => {
                let path = it.next().ok_or_else(|| "--file needs a path".to_string())?;
                out.file = Some(PathBuf::from(path));
            }
            other => return Err(format!("unknown option {other}, see homerow --help")),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(a: &[&str]) -> Result<Args, String> {
        parse(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn no_options_is_a_normal_run() {
        assert_eq!(p(&[]), Ok(Args::default()));
    }

    #[test]
    fn file_takes_a_path() {
        let a = p(&["--file", "src/main.rs"]).unwrap();
        assert_eq!(a.file, Some(PathBuf::from("src/main.rs")));
        assert_eq!(
            p(&["-f", "x.py"]).unwrap().file,
            Some(PathBuf::from("x.py"))
        );
    }

    #[test]
    fn demo_and_help() {
        assert!(p(&["--demo"]).unwrap().demo);
        assert!(p(&["-h"]).unwrap().help);
        assert!(p(&["--help"]).unwrap().help);
    }

    #[test]
    fn mistakes_are_reported_not_ignored() {
        assert_eq!(p(&["--file"]), Err("--file needs a path".to_string()));
        assert_eq!(
            p(&["--bogus"]),
            Err("unknown option --bogus, see homerow --help".to_string())
        );
    }

    #[test]
    fn decks_takes_a_path() {
        assert_eq!(
            p(&["--decks", "my.txt"]).unwrap().decks,
            Some(PathBuf::from("my.txt"))
        );
        assert_eq!(p(&["--decks"]), Err("--decks needs a path".to_string()));
    }
}
