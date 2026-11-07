// copy text to the system clipboard: the platform's own tool when there is
// one (run directly with the text on stdin, never through a shell), else the
// terminal clipboard escape code (OSC 52), which many terminals support
use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug, PartialEq)]
pub enum Copied {
    Tool(&'static str),
    Terminal,
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            // a short last chunk pads with '=' instead of encoding zeros
            if i <= chunk.len() {
                out.push(B64[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

// the escape code; base64 means nothing in the text can end it early
pub fn osc52(text: &str) -> String {
    format!("\u{1b}]52;c;{}\u{7}", base64(text.as_bytes()))
}

// clipboard tools to try, in order, for an os name like std::env::consts::OS
pub fn tools(os: &str) -> Vec<(&'static str, &'static [&'static str])> {
    match os {
        "macos" => vec![("pbcopy", &[])],
        "linux" | "freebsd" | "openbsd" | "netbsd" => vec![
            ("wl-copy", &[]),
            ("xclip", &["-selection", "clipboard"]),
            ("xsel", &["--clipboard", "--input"]),
        ],
        "windows" => vec![("clip", &[])],
        _ => Vec::new(),
    }
}

pub fn copy(text: &str) -> Copied {
    for (cmd, args) in tools(std::env::consts::OS) {
        if run(cmd, args, text) {
            return Copied::Tool(cmd);
        }
    }
    let mut out = std::io::stdout();
    let _ = out.write_all(osc52(text).as_bytes());
    let _ = out.flush();
    Copied::Terminal
}

fn run(cmd: &str, args: &[&str], text: &str) -> bool {
    let Ok(mut child) = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    let wrote = child
        .stdin
        .take()
        .is_some_and(|mut s| s.write_all(text.as_bytes()).is_ok());
    wrote && child.wait().is_ok_and(|st| st.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_standard_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"hello world"), "aGVsbG8gd29ybGQ=");
    }

    #[test]
    fn osc52_wraps_the_text_safely() {
        assert_eq!(osc52("hi\u{7}"), "\u{1b}]52;c;aGkH\u{7}");
    }

    #[test]
    fn each_platform_tries_its_own_tools() {
        assert_eq!(tools("macos")[0].0, "pbcopy");
        let linux: Vec<&str> = tools("linux").iter().map(|t| t.0).collect();
        assert_eq!(linux, vec!["wl-copy", "xclip", "xsel"]);
        assert_eq!(tools("windows")[0].0, "clip");
        assert!(tools("plan9").is_empty());
    }
}
