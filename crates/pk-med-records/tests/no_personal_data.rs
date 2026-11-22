//! The repository carries no personal data; a CLI's runtime output may.
//!
//! Walks every tracked file (`git ls-files` from the repository's root) for
//! the shapes personal data takes: phone and member numbers, Social Security
//! numbers, email addresses, street addresses, birth dates. Shapes, not a
//! denylist of real values, which would publish what it exists to keep out.
//! Dummy values follow fixed patterns (`crates/pk-med-fhir/fixtures/README.md`).

use std::process::Command;

fn repository_root() -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn tracked_files(root: &str) -> Vec<String> {
    let out = Command::new("git")
        .args(["ls-files"])
        .current_dir(root)
        .output()
        .expect("git ls-files");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

fn digit_runs(s: &str, min: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut run = String::new();
    for c in s.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() {
            run.push(c);
        } else {
            if run.len() >= min {
                out.push(run.clone());
            }
            run.clear();
        }
    }
    out
}

/// All one digit, zeros then one digit (`0000001`), or a 555 number.
fn is_dummy_number(n: &str) -> bool {
    let first = n.chars().next().unwrap();
    n.chars().all(|c| c == first)
        || n[..n.len() - 1].chars().all(|c| c == '0')
        || n.starts_with("555")
        || n.starts_with("1555")
}

/// `line` with each run of 32 or more lowercase hex digits that holds a
/// letter replaced by `#`: a digest (a pinned checksum, a commit), whose
/// digit runs are no one's number. A run of digits alone stays, and so does
/// a shorter run such as `bad1234567`. `#` is no separator the compact scan
/// drops, so the digits on either side never join.
fn without_digests(line: &str) -> String {
    fn flush(run: &mut String, out: &mut String) {
        if run.len() >= 32 && run.chars().any(|c| c.is_ascii_lowercase()) {
            out.push('#');
        } else {
            out.push_str(run);
        }
        run.clear();
    }
    let mut out = String::with_capacity(line.len());
    let mut run = String::new();
    for c in line.chars() {
        if c.is_ascii_digit() || ('a'..='f').contains(&c) {
            run.push(c);
        } else {
            flush(&mut run, &mut out);
            out.push(c);
        }
    }
    flush(&mut run, &mut out);
    out
}

fn scan_line(line: &str) -> Vec<String> {
    let mut findings = Vec::new();
    let line = &without_digests(line);
    let compact = line.replace(['(', ')', '-', '.', ' '], "");
    for n in digit_runs(line, 7).into_iter().chain(
        digit_runs(&compact, 10)
            .into_iter()
            .filter(|n| n.len() <= 11),
    ) {
        if !is_dummy_number(&n) {
            findings.push(format!("number {n}"));
        }
    }
    // ddd-dd-dddd
    let bytes = line.as_bytes();
    for i in 0..bytes.len().saturating_sub(10) {
        let w = &bytes[i..i + 11];
        let shape = w.iter().enumerate().all(|(j, b)| match j {
            3 | 6 => *b == b'-',
            _ => b.is_ascii_digit(),
        });
        let bounded = (i == 0 || !bytes[i - 1].is_ascii_digit())
            && bytes.get(i + 11).is_none_or(|b| !b.is_ascii_digit());
        if shape && bounded && &w[..3] != b"000" {
            findings.push(format!("ssn-like {}", String::from_utf8_lossy(w)));
        }
    }
    for word in line.split(|c: char| c.is_whitespace() || "\"'<>(),;`".contains(c)) {
        if let Some((local, domain)) = word.split_once('@') {
            // The host ends where a URL's path, port or query begins.
            let domain = domain
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '.'))
                .next()
                .unwrap_or_default()
                .trim_end_matches('.');
            let dummy = local.is_empty()
                || !domain.contains('.')
                || domain == "example.com"
                || domain.ends_with(".example")
                || domain.ends_with(".test")
                || domain.ends_with("users.noreply.github.com");
            if !dummy {
                findings.push(format!("email {word}"));
            }
        }
    }
    let lower = line.to_ascii_lowercase();
    for suffix in [
        " street", " st", " ave", " avenue", " blvd", " dr", " ln", " ct", " rd", " way",
    ] {
        let mut from = 0;
        while let Some(pos) = lower[from..].find(suffix) {
            let at = from + pos;
            let after = lower[at + suffix.len()..].chars().next();
            let word_ends = after.is_none_or(|c| !c.is_ascii_alphanumeric());
            let numbered = lower[..at]
                .split_whitespace()
                .rev()
                .take(3)
                .any(|w| w.len() >= 2 && w.chars().all(|c| c.is_ascii_digit()));
            if word_ends && numbered && !lower.contains("example st") {
                findings.push(format!("address-like `{}`", line.trim()));
            }
            from = at + suffix.len();
        }
    }
    // A full date of birth outside the dummy one.
    for key in ["birthdate", "birth_date", "dob"] {
        if let Some(pos) = lower.find(key) {
            let rest = &line[pos..];
            let date: String = rest
                .chars()
                .skip_while(|c| !c.is_ascii_digit())
                .take(10)
                .collect();
            if date.len() == 10 && date.as_bytes()[4] == b'-' && date != "1970-01-01" {
                findings.push(format!("birth date {date}"));
            }
        }
    }
    findings
}

#[test]
fn the_scanner_finds_the_shapes() {
    // Built here, not written out, so no tracked line carries the shapes.
    let positives = [
        (format!("call {}-{}-{}", "312", "867", "5309"), "number"),
        (format!("ssn {}-{}-{}", "123", "45", "6789"), "ssn-like"),
        (format!("mail someone@{}.com", "gmail"), "email"),
        (format!("https://someone@{}.com/callback", "gmail"), "email"),
        (
            format!("lives at {} Evergreen Terrace Dr", 742),
            "address-like",
        ),
        (format!(r#""birthDate": "{}-07-12""#, 1984), "birth date"),
        // Hex with a letter, but too short for a digest.
        (format!("bad{}", 1234567), "number"),
        // Digest length, but digits alone.
        ("12345678".repeat(4), "number"),
        // A digest beside a number keeps the number.
        (
            format!(
                "{} {}-{}-{}",
                "0123456789abcdef".repeat(4),
                "312",
                "867",
                "5309"
            ),
            "number",
        ),
    ];
    for (line, kind) in &positives {
        assert!(
            scan_line(line).iter().any(|f| f.starts_with(kind)),
            "{line}: {:?}",
            scan_line(line)
        );
    }
    let digest = "0123456789abcdef".repeat(4);
    for line in [
        "phone 555-0100",
        "MRN-0000001",
        "robin@example.com",
        "https://robin@pages.example.test/app/callback",
        "1 Example St",
        r#""birthDate": "1970-01-01""#,
        "NPI 0000000001",
        &format!("SHA256: {digest}"),
        &format!("commit {}", &digest[..40]),
    ] {
        assert!(scan_line(line).is_empty(), "{line}: {:?}", scan_line(line));
    }
}

#[test]
fn tracked_files_carry_no_personal_data() {
    let Some(root) = repository_root() else {
        // Not a git checkout (a packaged crate): nothing to scan.
        return;
    };
    let files = tracked_files(&root);
    assert!(!files.is_empty(), "git ls-files listed nothing");
    let mut findings = Vec::new();
    for file in &files {
        // This file holds the scanner's own synthetic positives.
        if file.ends_with(".lock")
            || file.starts_with("LICENSE")
            || file.ends_with("tests/no_personal_data.rs")
            || file.ends_with(".pdf")
        {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(format!("{root}/{file}")) else {
            continue;
        };
        for (i, line) in text.lines().enumerate() {
            // Base64 content (a Binary's `data`) is checked as the file it
            // decodes to, which is synthetic.
            if line.trim_start().starts_with("\"data\":") {
                continue;
            }
            for f in scan_line(line) {
                findings.push(format!("{file}:{}: {f}", i + 1));
            }
        }
    }
    assert!(
        findings.is_empty(),
        "personal-data shapes in tracked files:\n{}",
        findings.join("\n")
    );
}

#[test]
fn no_file_opts_out_of_the_pii_guard() {
    let Some(root) = repository_root() else {
        return;
    };
    for file in tracked_files(&root) {
        assert!(!file.ends_with(".pii-allowed"), "{file}");
    }
}
