use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{env, fs::{self, OpenOptions}, io::{self, Read, Seek, SeekFrom, Write}, path::PathBuf};

const MAX_MESSAGE: usize = 64 * 1024;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request { action: String, title: String, url: String }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config { index_path: PathBuf }
#[derive(Serialize)]
struct Response { ok: bool, #[serde(skip_serializing_if = "Option::is_none")] error: Option<String> }

fn invalid(s: impl Into<String>) -> io::Error { io::Error::new(io::ErrorKind::InvalidData, s.into()) }
fn read_frame(reader: &mut impl Read) -> io::Result<Option<Vec<u8>>> {
    let mut header = [0; 4];
    // A clean EOF is distinct from a truncated header.
    loop {
        match reader.read(&mut header[..1]) {
            Ok(0) => return Ok(None),
            Ok(_) => break,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    reader.read_exact(&mut header[1..])?;
    let len = u32::from_ne_bytes(header) as usize;
    if len == 0 || len > MAX_MESSAGE { return Err(invalid("Message length must be 1..65536 bytes")); }
    let mut body = vec![0; len];
    reader.read_exact(&mut body)?;
    Ok(Some(body))
}
fn write_frame(writer: &mut impl Write, response: &Response) -> io::Result<()> {
    let bytes = serde_json::to_vec(response)?;
    writer.write_all(&(bytes.len() as u32).to_ne_bytes())?;
    writer.write_all(&bytes)?;
    writer.flush()
}
fn config_path() -> io::Result<PathBuf> {
    let home = env::var_os("HOME").ok_or_else(|| invalid("HOME is not set"))?;
    Ok(PathBuf::from(home).join("Library/Application Support/SaveToMarkdown/config.json"))
}
fn load_destination() -> io::Result<PathBuf> {
    let config: Config = serde_json::from_slice(&fs::read(config_path()?)?)?;
    if !config.index_path.is_absolute() || config.index_path.file_name().and_then(|s| s.to_str()) != Some("Index.md") {
        return Err(invalid("index_path must be an absolute path ending in Index.md"));
    }
    Ok(config.index_path)
}
fn markdown_line(req: &Request) -> io::Result<String> {
    if req.action != "save" { return Err(invalid("Unsupported action")); }
    if req.url.chars().any(|c| c.is_control() || c.is_whitespace()) { return Err(invalid("URL contains whitespace or control characters")); }
    let parsed = url::Url::parse(&req.url).map_err(|_| invalid("Invalid URL"))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(invalid("Only HTTP and HTTPS links are supported"));
    }
    let cleaned = req.title.split_whitespace().collect::<Vec<_>>().join(" ");
    let cleaned: String = cleaned.chars().filter(|c| !c.is_control()).collect();
    let title = if cleaned.is_empty() { &req.url } else { &cleaned };
    let mut escaped = String::new();
    for c in title.chars() {
        if r"\`*_{}[]<>()!#".contains(c) { escaped.push('\\'); }
        if c == '&' { escaped.push_str("&amp;"); } else { escaped.push(c); }
    }
    // Preserve the original URL, encoding only Markdown destination delimiters.
    let dest = req.url.replace('\\', "%5C").replace('(', "%28").replace(')', "%29")
        .replace('<', "%3C").replace('>', "%3E");
    Ok(format!("- [{escaped}]({dest})\n"))
}
fn save(req: &Request) -> io::Result<()> {
    let line = markdown_line(req)?;
    let path = load_destination()?;
    // Do not create the vault or truncate existing contents. Locks serialize host instances.
    let mut file = OpenOptions::new().read(true).append(true).create(true).open(path)?;
    file.lock_exclusive()?;
    let len = file.metadata()?.len();
    let mut entry = String::new();
    if len > 0 {
        file.seek(SeekFrom::End(-1))?;
        let mut last = [0];
        file.read_exact(&mut last)?;
        if last[0] != b'\n' { entry.push('\n'); }
    }
    entry.push_str(&line);
    file.write_all(entry.as_bytes())?;
    file.sync_all()?;
    // Dropping the file releases the lock, including on errors.
    Ok(())
}
fn run() -> io::Result<()> {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    while let Some(bytes) = read_frame(&mut input)? {
        let result = serde_json::from_slice::<Request>(&bytes).map_err(io::Error::from).and_then(|r| save(&r));
        let response = match result {
            Ok(()) => Response { ok: true, error: None },
            Err(e) => Response { ok: false, error: Some(e.to_string()) },
        };
        write_frame(&mut output, &response)?;
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() { eprintln!("Save to Markdown: {e}"); std::process::exit(1); }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn framing() {
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &Response { ok: true, error: None }).unwrap();
        let mut input = &bytes[..];
        assert_eq!(read_frame(&mut input).unwrap().unwrap(), br#"{"ok":true}"#);
        assert!(read_frame(&mut input).unwrap().is_none());
        assert!(read_frame(&mut &bytes[..2]).is_err());
        assert!(read_frame(&mut &bytes[..bytes.len()-1]).is_err());
        assert!(read_frame(&mut &((MAX_MESSAGE as u32)+1).to_ne_bytes()[..]).is_err());
    }
    #[test]
    fn markdown() {
        let r = Request { action: "save".into(), title: "A [title]\n& B".into(), url: "https://example.com/a(b)".into() };
        assert_eq!(markdown_line(&r).unwrap(), "- [A \\[title\\] &amp; B](https://example.com/a%28b%29)\n");
        assert!(serde_json::from_str::<Request>(r#"{"action":"save","title":"T","url":"https://example.com","file":"/tmp/x"}"#).is_err());
        assert!(markdown_line(&Request { url: "javascript:alert(1)".into(), ..r }).is_err());
    }
}
